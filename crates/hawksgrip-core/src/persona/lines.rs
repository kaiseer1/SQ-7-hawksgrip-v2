//! Theatrical line pools. Placeholders: {brg} {rng_km} {eyes} {band} {vis_km}.
//!
//! Kindah speaks only about what she perceives and does in Phases 0–3:
//! watching, classifying, converging, locking. Interception is not modelled,
//! and her breach lines say so.

pub const IDLE: usize = 0;
pub const DETECTED: usize = 1;
pub const CONFIRMED: usize = 2;
pub const CLASS_DRONE: usize = 3;
pub const CLASS_BIRD: usize = 4;
pub const CONVERGENCE: usize = 5;
pub const LOCKED: usize = 6;
pub const LOST: usize = 7;
pub const REACQUIRED: usize = 8;
pub const REEXAMINED: usize = 9;
pub const BAND_DARK: usize = 10;
pub const BAND_CLEAR: usize = 11;
pub const STORM: usize = 12;
pub const CLEAR_SKY: usize = 13;
pub const BREACH: usize = 14;
pub const CHANT: usize = 15;
pub const POOLS: usize = 16;

pub const THEATRICAL: [&[&str]; POOLS] = [
    // IDLE
    &[
        "My twenty-four eyes drink the horizon. Nothing breathes in this sky without my permission.",
        "I circle my throne and I do not hurry. The sky hurries for me.",
        "Below me they burn oil and call it power. I am lightning, and I do not smoke.",
        "Every grain of this desert has been counted. Every shadow has been weighed.",
        "Seven daughters beneath me, twenty-four eyes upon the world. Sleep, refinery. I do not.",
        "Combustion is the breath of peasants. I hum on pure current, and I never tire.",
        "Let them come. The sky is my mirror, and every intruder will find my face in it.",
    ],
    // DETECTED
    &[
        "Something stirs at {rng_km} kilometres, bearing {brg}. I have noticed it. That is already more than it deserves.",
        "A speck on my horizon, bearing {brg}. One eye turns. One is enough to begin.",
        "Movement over my sands, {rng_km} kilometres out. Hold still, little thing.",
        "A shape crawls at the edge of my sky. Let me look closer.",
        "Bearing {brg}. Something small believes it is unseen.",
    ],
    // CONFIRMED
    &[
        "It is real. It persists. How tiresome.",
        "Twice it has shown itself. It should have hidden better.",
        "Confirmed. Something has entered my court without bowing.",
        "Not a trick of the heat. A trespasser.",
    ],
    // CLASS_DRONE
    &[
        "Not a bird. A machine, and a crude one, buzzing over my refinery like a fly over a feast.",
        "Rotors, not wings. A drone. Unbidden and unwelcome.",
        "Mechanical. It hums with borrowed current and pretends to be brave.",
        "A drone. The desert did not send this. Someone did.",
    ],
    // CLASS_BIRD
    &[
        "Only a bird. Fly on, child of the desert. You were here before my kingdom.",
        "Feathers, not rotors. I forgive the falcon what I would never forgive a machine.",
        "A bird. My eyes return to their watch.",
        "Wings that beat like a heart. Not my concern.",
    ],
    // CONVERGENCE
    &[
        "What is this filth crawling across my sky? Twenty-four times I look upon you, little locust, and twenty-four times I find you pathetic.",
        "All my eyes, now. Look at it. Look at what thinks it can approach me.",
        "Every eye I own is upon you. There is nowhere in my sky that I am not already looking.",
        "She looks at you. All of her.",
    ],
    // LOCKED
    &[
        "Held. {eyes} eyes, and not one of them blinks.",
        "I have you, bearing {brg}, {rng_km} kilometres. You are mine to watch now.",
        "Locked. Every turn you make is written in my sight before you make it.",
    ],
    // LOST
    &[
        "It slipped behind the stone. The desert hides cowards well, but never for long.",
        "Gone from my sight. Search, my eyes. It cannot have gone far.",
        "The rock swallowed it. Rocks do not keep secrets from me forever.",
    ],
    // REACQUIRED
    &[
        "There you are again. Did you think the rock would shelter you from me?",
        "Found. I never stopped looking.",
        "Back in my sight, bearing {brg}. As I knew you would be.",
    ],
    // REEXAMINED
    &[
        "Come closer, little bird. Let me be certain of you.",
        "A second look. A queen does not guess twice.",
        "You are near my house now. I will look at you again.",
    ],
    // BAND_DARK (switched to SWIR/LWIR)
    &[
        "The Shamal blinds the common eye. I see in heat now. Even dust cannot hide a warm heart.",
        "Dust, everywhere dust. My eyes turn to the infrared, and the storm becomes glass.",
        "They hoped the sand would veil them. I see in {band}.",
    ],
    // BAND_CLEAR (back to visible)
    &[
        "The storm passes. Light returns to my eyes, sharper than before.",
        "Clear air. My eyes return to the visible, and the horizon is mine again.",
    ],
    // STORM (visibility dropped)
    &[
        "The north wind brings the sand. Let it. I have watched storms bow before.",
        "The Shamal rises. Visibility {vis_km} kilometres. A lesser queen would be afraid.",
    ],
    // CLEAR_SKY (visibility restored)
    &[
        "The air clears. My sky is clean again.",
        "The dust settles at my feet, where it belongs.",
    ],
    // BREACH
    &[
        "It touched the refinery. Note it, scribes: in this trial my eyes watched, and that is all they were permitted to do.",
        "An insult has reached my house. This trial measures my sight, not my answer.",
        "Recorded. I saw it all the way in. The rest of my court is not yet in this trial.",
    ],
    // CHANT (the Seven)
    &[
        "ALL HAIL THE QUEEN! MY WINGS AND MY LIGHTNING ARE YOURS!",
        "WE HOLD THE RING BENEATH YOU, KINDAH! NONE PASS BELOW OUR QUEEN!",
        "HER SKY, HER LAW! THE SEVEN STAND READY!",
        "WE ARE THE SEVEN! WE LIVE IN HER SHADOW AND HER LIGHT!",
        "SHE SEES ALL! WE GUARD ALL!",
        "NO SMOKE, NO FIRE, ONLY HER LIGHTNING!",
    ],
];
