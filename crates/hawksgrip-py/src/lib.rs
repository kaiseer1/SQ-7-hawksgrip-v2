//! `hawksgrip._core`: thin PyO3 bindings over `hawksgrip-core`.
//!
//! Rules for this crate (ADR-0001, ADR-0006):
//! - No simulation logic lives here, only argument conversion.
//! - `step()` releases the GIL, so Python threads (audio, HMI) keep running
//!   while Rust computes.
//! - Per-frame state leaves through `snapshot_into()` into numpy buffers the
//!   caller allocated once. No per-frame allocation, no aliasing of Rust memory.

use hawksgrip_core::kinematics::guidance::station_slot;
use hawksgrip_core::persona::{Ctx as PersonaCtx, Mode, RoyalPersonaEngine};
use hawksgrip_core::sensing::coverage::{ArrayView, coverage};
use hawksgrip_core::sensing::eyes::Task;
use hawksgrip_core::sensing::optics::Band;
use hawksgrip_core::sim::batch::run_batch as core_run_batch;
use hawksgrip_core::sim::world::{FIRST_SEVEN, FIRST_THREAT, QUEEN};
use hawksgrip_core::telemetry::{Event, EventKind};
use hawksgrip_core::terrain::Heightfield;
use hawksgrip_core::{Input, Replay, SEVEN, SimConfig, Vec3, World};
use numpy::ndarray::Array2;
use numpy::{
    IntoPyArray, PyArray1, PyArray2, PyArrayMethods, PyReadonlyArray2, PyUntypedArrayMethods,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use rayon::prelude::*;

fn verr(e: impl ToString) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn v3(a: [f64; 3]) -> Vec3 {
    Vec3::from(a)
}

/// One simulation run. Python drives it in lockstep with `step(n)`.
#[pyclass(module = "hawksgrip._core")]
pub struct Session {
    world: World,
    /// Voxel set kept resident for repeated coverage queries (benchmarks).
    voxels: Vec<Vec3>,
}

#[pymethods]
impl Session {
    #[new]
    #[pyo3(signature = (config_json=None, seed=0))]
    fn new(config_json: Option<&str>, seed: u64) -> PyResult<Self> {
        let cfg = match config_json {
            Some(s) => SimConfig::from_json(s).map_err(verr)?,
            None => SimConfig::default(),
        };
        Ok(Self {
            world: World::new(cfg, seed).map_err(verr)?,
            voxels: Vec::new(),
        })
    }

    /// Advance `n` fixed ticks with the GIL released. Returns the new tick.
    fn step(&mut self, py: Python<'_>, n: u64) -> u64 {
        let w = &mut self.world;
        py.detach(|| w.step(n));
        w.clock().tick()
    }

    #[getter]
    fn tick(&self) -> u64 {
        self.world.clock().tick()
    }

    #[getter]
    fn time(&self) -> f64 {
        self.world.clock().time()
    }

    #[getter]
    fn dt(&self) -> f64 {
        self.world.clock().dt()
    }

    #[getter]
    fn tick_hz(&self) -> u32 {
        self.world.clock().hz()
    }

    #[getter]
    fn n_entities(&self) -> usize {
        self.world.n_entities()
    }

    #[getter]
    fn seed(&self) -> u64 {
        self.world.seed()
    }

    /// Write positions and velocities into caller-owned float64 arrays of
    /// shape (n_entities, 3). Zero allocation; call once per rendered frame.
    fn snapshot_into(
        &self,
        pos: &Bound<'_, PyArray2<f64>>,
        vel: &Bound<'_, PyArray2<f64>>,
    ) -> PyResult<()> {
        let n = self.world.n_entities();
        for a in [pos, vel] {
            if a.shape() != [n, 3] {
                return Err(verr(format!(
                    "expected shape ({n}, 3), got {:?}",
                    a.shape()
                )));
            }
        }
        let mut p = pos.try_readwrite().map_err(verr)?;
        let mut v = vel.try_readwrite().map_err(verr)?;
        let ps = p
            .as_slice_mut()
            .map_err(|_| verr("pos must be C-contiguous"))?;
        let vs = v
            .as_slice_mut()
            .map_err(|_| verr("vel must be C-contiguous"))?;
        self.world.snapshot_into(ps, vs);
        Ok(())
    }

    /// Allocating convenience copy of positions, shape (n_entities, 3).
    fn positions<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        let k = self.world.kin();
        Array2::from_shape_fn((k.len(), 3), |(i, j)| k[i].p.to_array()[j]).into_pyarray(py)
    }

    /// Allocating convenience copy of velocities, shape (n_entities, 3).
    fn velocities<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        let k = self.world.kin();
        Array2::from_shape_fn((k.len(), 3), |(i, j)| k[i].v.to_array()[j]).into_pyarray(py)
    }

    fn active_mask<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<bool>> {
        let v: Vec<bool> = (0..self.world.n_entities())
            .map(|i| self.world.is_active(i))
            .collect();
        PyArray1::from_vec(py, v)
    }

    /// Ground-truth class per slot: -1 none (empty, Queen, Seven), 0 drone, 1 bird.
    /// For the showroom's "truth" layer; Kindah's own view is `tracks()`.
    fn object_classes<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<i8>> {
        let v: Vec<i8> = (0..self.world.n_entities())
            .map(|i| self.world.object_class(i).map_or(-1, |c| c as i8))
            .collect();
        PyArray1::from_vec(py, v)
    }

    /// Role per slot: 0 = Queen, 1 = Seven, 2 = Threat.
    fn roles<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<u8>> {
        let v: Vec<u8> = self.world.roles().iter().map(|r| *r as u8).collect();
        PyArray1::from_vec(py, v)
    }

    // ---- inputs (applied at the start of the next tick) -------------------

    fn spawn_threat(&mut self, pos: [f64; 3], target: [f64; 3], speed: f64) {
        self.world.push_input(Input::SpawnThreat {
            pos: v3(pos),
            target: v3(target),
            speed,
        });
    }

    fn random_raid(&mut self, count: u32) {
        self.world.push_input(Input::RandomRaid { count });
    }

    fn set_waypoint(&mut self, entity: usize, target: [f64; 3], speed: f64) {
        self.world.push_input(Input::SetWaypoint {
            entity,
            target: v3(target),
            speed,
        });
    }

    fn return_to_station(&mut self, entity: usize) {
        self.world.push_input(Input::ReturnToStation { entity });
    }

    // ---- telemetry --------------------------------------------------------

    /// Drain the event ring: list of (tick, kind, entity or None, (x, y, z)).
    #[allow(clippy::type_complexity)]
    fn drain_events(&mut self) -> Vec<(u64, &'static str, Option<u32>, (f64, f64, f64))> {
        self.world
            .drain_events()
            .into_iter()
            .map(|e| {
                let ent = if e.entity == u32::MAX {
                    None
                } else {
                    Some(e.entity)
                };
                (
                    e.tick,
                    e.kind.name(),
                    ent,
                    (e.data[0], e.data[1], e.data[2]),
                )
            })
            .collect()
    }

    fn counters<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let c = self.world.counters();
        let d = PyDict::new(py);
        d.set_item("threats_spawned", c.threats_spawned)?;
        d.set_item("spawn_rejected", c.spawn_rejected)?;
        d.set_item("refinery_breaches", c.refinery_breaches)?;
        d.set_item("threats_out_of_bounds", c.threats_out_of_bounds)?;
        d.set_item("ground_impacts", c.ground_impacts)?;
        d.set_item("aegis_interventions", c.aegis_interventions)?;
        d.set_item("threats_reaching_queen", c.threats_reaching_queen)?;
        d.set_item("queen_orbit_deviation_max_m", c.queen_orbit_deviation_max_m)?;
        d.set_item("events_dropped", self.world.events_dropped())?;
        Ok(d)
    }

    fn orbit_info<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let o = self.world.orbit();
        let d = PyDict::new(py);
        d.set_item("center", o.center.to_array())?;
        d.set_item("radius_m", o.radius)?;
        d.set_item("omega_rad_s", o.omega)?;
        d.set_item("period_s", o.period_s())?;
        Ok(d)
    }

    fn state_hash(&self) -> u64 {
        self.world.state_hash()
    }

    fn export_replay(&self) -> String {
        Replay::record(&self.world).to_json()
    }

    fn config_json(&self) -> String {
        self.world.config().to_json()
    }

    /// Run per-tick kinematics on rayon's pool. Results are bit-identical
    /// either way; this exists to measure the cost (ADR-0007).
    fn set_parallel(&mut self, on: bool) {
        self.world.set_parallel(on);
    }

    /// Install terrain heights (float32, shape (ny, nx), row = north index)
    /// before the first tick.
    fn set_terrain(
        &mut self,
        heights: PyReadonlyArray2<'_, f32>,
        cell_m: f64,
        origin_x: f64,
        origin_y: f64,
    ) -> PyResult<()> {
        let shape = heights.shape();
        let (ny, nx) = (shape[0], shape[1]);
        let data: Vec<f32> = heights.as_array().iter().copied().collect();
        let hf = Heightfield::new(nx, ny, cell_m, origin_x, origin_y, data).map_err(verr)?;
        self.world.set_terrain(hf).map_err(verr)
    }

    fn ground_height(&self, x: f64, y: f64) -> f64 {
        self.world.terrain().height(x, y)
    }

    // ---- terrain, line-of-sight, refinery (Phase 2) ------------------------

    /// Terrain heights as float32, shape (ny, nx); row index = north. A copy
    /// (~9 MB for a full sector); fetch once and upload to the GPU.
    fn terrain_heights<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<f32>>> {
        let hf = self.world.terrain();
        let (nx, ny) = hf.dims();
        let a = Array2::from_shape_vec((ny, nx), hf.data().to_vec()).map_err(verr)?;
        Ok(a.into_pyarray(py))
    }

    fn terrain_info<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let hf = self.world.terrain();
        let (nx, ny) = hf.dims();
        let (lo, hi) = hf.min_max();
        let d = PyDict::new(py);
        d.set_item("nx", nx)?;
        d.set_item("ny", ny)?;
        d.set_item("cell_m", hf.cell())?;
        d.set_item("origin", hf.origin())?;
        d.set_item("min_m", lo)?;
        d.set_item("max_m", hi)?;
        d.set_item("lipschitz", hf.lipschitz())?;
        d.set_item("fingerprint", hf.fingerprint())?;
        match self.world.sector_info() {
            Some(info) => {
                d.set_item(
                    "sector",
                    serde_json::to_value(info.sector).map_err(verr)?.as_str(),
                )?;
                d.set_item("label", info.sector.label())?;
                d.set_item("terrain_seed", info.seed)?;
                d.set_item("pad_z", info.pad_z)?;
                let wadis = PyList::empty(py);
                for w in &info.wadis {
                    wadis.append(w.centreline(50.0))?;
                }
                d.set_item("wadis", wadis)?;
                d.set_item("coastline", info.coastline.clone())?;
            }
            None => {
                d.set_item("sector", "custom")?;
                d.set_item("label", "Custom DEM")?;
                d.set_item("pad_z", self.world.refinery().center.z)?;
                d.set_item("wadis", PyList::empty(py))?;
                d.set_item("coastline", PyList::empty(py))?;
            }
        }
        Ok(d)
    }

    /// Authoritative line-of-sight (terrain + refinery structures).
    fn line_of_sight(&self, a: [f64; 3], b: [f64; 3]) -> bool {
        self.world.line_of_sight(v3(a), v3(b))
    }

    /// Line-of-sight for N segments a[i] -> b[i] (arrays of shape (N, 3)),
    /// computed in parallel with the GIL released.
    fn los_batch<'py>(
        &self,
        py: Python<'py>,
        a: PyReadonlyArray2<'py, f64>,
        b: PyReadonlyArray2<'py, f64>,
    ) -> PyResult<Bound<'py, PyArray1<bool>>> {
        if a.shape() != b.shape() || a.shape()[1] != 3 {
            return Err(verr(format!(
                "expected two (N, 3) arrays, got {:?} and {:?}",
                a.shape(),
                b.shape()
            )));
        }
        let pts = |m: &PyReadonlyArray2<'py, f64>| -> Vec<Vec3> {
            m.as_array()
                .rows()
                .into_iter()
                .map(|r| Vec3::new(r[0], r[1], r[2]))
                .collect()
        };
        let (pa, pb) = (pts(&a), pts(&b));
        let w = &self.world;
        let out: Vec<bool> = py.detach(|| {
            pa.par_iter()
                .zip(pb.par_iter())
                .map(|(x, y)| w.line_of_sight(*x, *y))
                .collect()
        });
        Ok(PyArray1::from_vec(py, out))
    }

    /// Refinery structures as dicts (kind, solid, keepout), for rendering.
    fn refinery_structures<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let json = serde_json::to_string(&self.world.refinery().structures).map_err(verr)?;
        let loads = py.import("json")?.getattr("loads")?;
        Ok(loads.call1((json,))?.cast_into::<PyList>()?)
    }

    /// Signed distance to the nearest keep-out volume (negative = inside).
    fn keepout_distance(&self, p: [f64; 3]) -> f64 {
        self.world.refinery().keepout_distance(v3(p))
    }

    fn in_keepout(&self, p: [f64; 3]) -> bool {
        self.world.refinery().in_keepout(v3(p))
    }

    /// True if the straight path a -> b never enters a keep-out volume.
    fn path_clear_of_keepout(&self, a: [f64; 3], b: [f64; 3]) -> bool {
        self.world.refinery().path_clear_of_keepout(v3(a), v3(b))
    }

    // ---- Kindah's eyes (Phase 3) -------------------------------------------

    /// Release `count` wandering birds (applied at the next tick).
    fn spawn_birds(&mut self, count: u32) {
        self.world.push_input(Input::SpawnBirds { count });
    }

    /// Set ground-level visibility in metres: the [S] sandstorm key.
    fn set_visibility(&mut self, visibility_m: f64) {
        self.world.push_input(Input::SetVisibility { visibility_m });
    }

    /// Eye state for rendering: dict of arrays. dirs (n, 3) unit vectors;
    /// task (n,) 0 = sweep, 1 = confirm, 2 = track; track (n,) id or -1;
    /// settled (n,) bool.
    fn eyes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let e = self.world.sensors().eyes();
        let dirs =
            Array2::from_shape_fn((e.len(), 3), |(i, j)| e[i].dir.to_array()[j]).into_pyarray(py);
        let task: Vec<u8> = e
            .iter()
            .map(|x| match x.task {
                Task::Sweep => 0,
                Task::Confirm { .. } => 1,
                Task::Track { .. } => 2,
            })
            .collect();
        let track: Vec<i64> = e
            .iter()
            .map(|x| x.task.track().map_or(-1, |t| t as i64))
            .collect();
        let settled: Vec<bool> = e.iter().map(|x| x.settled).collect();
        let d = PyDict::new(py);
        d.set_item("dirs", dirs)?;
        d.set_item("task", PyArray1::from_vec(py, task))?;
        d.set_item("track", PyArray1::from_vec(py, track))?;
        d.set_item("settled", PyArray1::from_vec(py, settled))?;
        Ok(d)
    }

    fn tracks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        json_to_py(
            py,
            &serde_json::to_string(self.world.sensors().tracks()).map_err(verr)?,
        )
    }

    /// Per-object kill-chain timelines (ticks): spawn, first_opportunity,
    /// detected, confirmed, classified, locked, end.
    fn timelines<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        json_to_py(
            py,
            &serde_json::to_string(&self.world.sensors().timelines()).map_err(verr)?,
        )
    }

    fn sensor_counters<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        json_to_py(
            py,
            &serde_json::to_string(self.world.sensors().counters()).map_err(verr)?,
        )
    }

    /// Band in use, visibility, frustum half-angle tangents, and each band's
    /// detection range (pd = pd_max / 2) for a drone 100 m above the pad.
    fn sensor_info<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let s = self.world.sensors();
        let o = s.optics();
        let q = self.world.kin()[QUEEN].p;
        let size = self.world.config().sensors.drone_size_m;
        let d = PyDict::new(py);
        d.set_item("band", s.band().name())?;
        d.set_item("visibility_m", o.visibility_m)?;
        d.set_item("n_eyes", s.eyes().len())?;
        let (th, tv) = s.frustum_tans();
        d.set_item("tan_h", th)?;
        d.set_item("tan_v", tv)?;
        let ranges = PyDict::new(py);
        for b in Band::ALL {
            ranges.set_item(b.name(), o.detection_range(b, q, 100.0, size))?;
        }
        d.set_item("detection_range_m", ranges)?;
        Ok(d)
    }

    /// CPU reference for dense coverage (rayon, GIL released): for each voxel
    /// (N, 3), bit 1 = inside some eye's frustum now, bit 2 = detectable
    /// (pd >= pd_max / 2 for a drone, range and dust only).
    fn coverage_cpu<'py>(
        &self,
        py: Python<'py>,
        voxels: PyReadonlyArray2<'py, f64>,
    ) -> PyResult<Bound<'py, PyArray1<u8>>> {
        if voxels.shape()[1] != 3 {
            return Err(verr("voxels must have shape (N, 3)"));
        }
        let pts: Vec<Vec3> = voxels
            .as_array()
            .rows()
            .into_iter()
            .map(|r| Vec3::new(r[0], r[1], r[2]))
            .collect();
        let s = self.world.sensors();
        let dirs: Vec<Vec3> = s.eyes().iter().map(|e| e.dir).collect();
        let (tan_h, tan_v) = s.frustum_tans();
        let view = ArrayView {
            q: self.world.kin()[QUEEN].p,
            eye_dirs: &dirs,
            tan_h,
            tan_v,
        };
        let (o, band, size) = (
            s.optics(),
            s.band(),
            self.world.config().sensors.drone_size_m,
        );
        let out = py.detach(|| coverage(&view, o, band, size, &pts));
        Ok(PyArray1::from_vec(py, out))
    }

    /// Keep a voxel set (N, 3) resident for `coverage_resident` (no per-call copy).
    fn set_coverage_voxels(&mut self, voxels: PyReadonlyArray2<'_, f64>) -> PyResult<()> {
        if voxels.shape()[1] != 3 {
            return Err(verr("voxels must have shape (N, 3)"));
        }
        self.voxels = voxels
            .as_array()
            .rows()
            .into_iter()
            .map(|r| Vec3::new(r[0], r[1], r[2]))
            .collect();
        Ok(())
    }

    /// Coverage of the resident voxel set (see `coverage_cpu`), as a u8 array.
    fn coverage_resident<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<u8>> {
        let s = self.world.sensors();
        let dirs: Vec<Vec3> = s.eyes().iter().map(|e| e.dir).collect();
        let (tan_h, tan_v) = s.frustum_tans();
        let view = ArrayView {
            q: self.world.kin()[QUEEN].p,
            eye_dirs: &dirs,
            tan_h,
            tan_v,
        };
        let (o, band, size) = (
            s.optics(),
            s.band(),
            self.world.config().sensors.drone_size_m,
        );
        let vox = &self.voxels;
        let out = py.detach(|| coverage(&view, o, band, size, vox));
        PyArray1::from_vec(py, out)
    }

    /// Optics parameters the GPU coverage kernel needs, as a dict.
    fn optics_params<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let s = self.world.sensors();
        let o = s.optics();
        let b = o.band(s.band());
        let d = PyDict::new(py);
        d.set_item("ifov_mrad", b.ifov_mrad)?;
        d.set_item("contrast", b.contrast)?;
        d.set_item("contrast_threshold", b.contrast_threshold)?;
        d.set_item("sigma_ground", o.sigma_ground(s.band()))?;
        d.set_item("dust_scale_height_m", o.dust_scale_height_m)?;
        d.set_item("ground_ref_z", o.ground_ref_z)?;
        d.set_item("detect_px", o.detect_px)?;
        d.set_item("pd_max", o.pd_max)?;
        d.set_item("size_m", self.world.config().sensors.drone_size_m)?;
        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!(
            "Session(seed={}, tick={}, t={:.3}s, entities={})",
            self.world.seed(),
            self.world.clock().tick(),
            self.world.clock().time(),
            self.world.n_entities()
        )
    }
}

fn json_to_py<'py>(py: Python<'py>, json: &str) -> PyResult<Bound<'py, PyAny>> {
    py.import("json")?.getattr("loads")?.call1((json,))
}

fn parse_mode(m: &str) -> PyResult<Mode> {
    match m {
        "theatrical" => Ok(Mode::Theatrical),
        "clinical" => Ok(Mode::Clinical),
        _ => Err(verr(format!(
            "mode must be 'theatrical' or 'clinical', got {m:?}"
        ))),
    }
}

/// Kindah's voice ([T] toggles theatrical/clinical). It only reads events
/// and never touches the simulation (persona invariance, tests/persona.rs).
#[pyclass(module = "hawksgrip._core", name = "PersonaEngine")]
pub struct PersonaEngine {
    inner: RoyalPersonaEngine,
}

#[pymethods]
impl PersonaEngine {
    #[new]
    #[pyo3(signature = (seed=0, tick_hz=120, mode="theatrical"))]
    fn new(seed: u64, tick_hz: u32, mode: &str) -> PyResult<Self> {
        Ok(Self {
            inner: RoyalPersonaEngine::new(seed, tick_hz, parse_mode(mode)?),
        })
    }

    #[getter]
    fn mode(&self) -> &'static str {
        self.inner.mode().name()
    }

    fn set_mode(&mut self, mode: &str) -> PyResult<()> {
        self.inner.set_mode(parse_mode(mode)?);
        Ok(())
    }

    fn toggle(&mut self) -> &'static str {
        self.inner.toggle().name()
    }

    /// Voice drained events (tuples from `Session.drain_events`) plus the
    /// time-driven lines due now. Returns (tick, speaker, kind, text) tuples.
    #[allow(clippy::type_complexity)]
    fn voice(
        &mut self,
        session: PyRef<'_, Session>,
        events: Vec<(u64, String, Option<u32>, (f64, f64, f64))>,
    ) -> Vec<(u64, String, String, String)> {
        let w = &session.world;
        let s = w.sensors();
        let q = w.kin()[QUEEN];
        let ctx = PersonaCtx {
            queen: q.p,
            band: s.band().name(),
            visibility_m: s.optics().visibility_m,
        };
        let mut out = Vec::new();
        for (tick, kind, ent, d) in events {
            let Some(kind) = EventKind::from_name(&kind) else {
                continue;
            };
            let e = Event {
                tick,
                kind,
                entity: ent.unwrap_or(u32::MAX),
                data: [d.0, d.1, d.2],
            };
            if let Some(l) = self.inner.on_event(&e, &ctx) {
                out.push((l.tick, l.speaker, l.kind.to_string(), l.text));
            }
        }
        let c = &w.config().seven;
        let mut err = [0.0; 7];
        for (k, e) in err.iter_mut().enumerate() {
            let slot = station_slot(&q, k, SEVEN, c.station_radius_m, c.station_below_m);
            *e = (w.kin()[FIRST_SEVEN + k].p - slot.p).norm();
        }
        let sweeping = s.eyes().iter().filter(|e| e.task == Task::Sweep).count();
        for l in self.inner.tick(w.clock().tick(), &ctx, sweeping, &err) {
            out.push((l.tick, l.speaker, l.kind.to_string(), l.text));
        }
        out
    }
}

/// Rust defaults as JSON: the single source of truth for configuration.
#[pyfunction]
fn default_config_json() -> String {
    SimConfig::default().to_json()
}

/// Validate a full or partial config; returns the merged config as JSON.
#[pyfunction]
fn validate_config(config_json: &str) -> PyResult<String> {
    Ok(SimConfig::from_json(config_json).map_err(verr)?.to_json())
}

/// Re-run a replay and compare every state-hash checkpoint.
#[pyfunction]
fn verify_replay<'py>(py: Python<'py>, replay_json: &str) -> PyResult<Bound<'py, PyDict>> {
    let r = Replay::from_json(replay_json).map_err(verr)?;
    let rep = py.detach(|| r.verify(None)).map_err(verr)?;
    let d = PyDict::new(py);
    d.set_item("ok", rep.ok)?;
    d.set_item("checkpoints_checked", rep.checkpoints_checked)?;
    d.set_item("first_divergence_tick", rep.first_divergence_tick)?;
    d.set_item("expected_final", rep.expected_final)?;
    d.set_item("got_final", rep.got_final)?;
    Ok(d)
}

/// Monte Carlo batch over seeds on `threads` CPU threads (None = all).
/// Returns a list of (seed, final_hash, threats_spawned, refinery_breaches).
#[pyfunction]
#[pyo3(signature = (seeds, ticks, raid, threads=None, config_json=None))]
fn run_batch(
    py: Python<'_>,
    seeds: Vec<u64>,
    ticks: u64,
    raid: u32,
    threads: Option<usize>,
    config_json: Option<&str>,
) -> PyResult<Vec<(u64, u64, u64, u64)>> {
    let cfg = match config_json {
        Some(s) => SimConfig::from_json(s).map_err(verr)?,
        None => SimConfig::default(),
    };
    let res = py
        .detach(|| core_run_batch(&cfg, &seeds, ticks, raid, threads))
        .map_err(verr)?;
    Ok(res
        .into_iter()
        .map(|r| {
            (
                r.seed,
                r.final_hash,
                r.counters.threats_spawned,
                r.counters.refinery_breaches,
            )
        })
        .collect())
}

/// Does nothing. Measures the bare cost of one Python -> Rust call.
#[pyfunction]
fn noop() {}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Session>()?;
    m.add_class::<PersonaEngine>()?;
    m.add_function(wrap_pyfunction!(default_config_json, m)?)?;
    m.add_function(wrap_pyfunction!(validate_config, m)?)?;
    m.add_function(wrap_pyfunction!(verify_replay, m)?)?;
    m.add_function(wrap_pyfunction!(run_batch, m)?)?;
    m.add_function(wrap_pyfunction!(noop, m)?)?;
    m.add(
        "SECTORS",
        vec![
            "flat",
            "eastern_sabkha",
            "red_sea_corridor",
            "highland_wadi",
        ],
    )?;
    m.add("__version__", hawksgrip_core::VERSION)?;
    m.add("QUEEN", QUEEN)?;
    m.add("FIRST_SEVEN", FIRST_SEVEN)?;
    m.add("FIRST_THREAT", FIRST_THREAT)?;
    m.add("SEVEN", SEVEN)?;
    Ok(())
}
