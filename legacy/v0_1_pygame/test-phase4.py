"""
Phase 4 Test - Verify interceptors and butterfly formation.
Run from project root: python test_phase4.py
"""

import config
from agents import Mothership, Interceptor
from systems import formation


def test_formation():
    print("=== Phase 4 Test: Interceptors & Butterfly Formation ===\n")
    
    # Create mothership
    mothership = Mothership()
    print(f"Mothership at: {mothership.position}")
    print()
    
    # Create interceptors in formation
    interceptors = formation.create_interceptors_in_formation(
        mothership.position,
        count=config.INTERCEPTOR_COUNT
    )
    
    print(f"Created {len(interceptors)} interceptors in butterfly formation:\n")
    print(f"{'ID':<16} {'Role':<12} {'Position':<24} {'Fuel':<6} {'State'}")
    print("-" * 70)
    
    for i, intc in enumerate(interceptors):
        role = formation.get_formation_role(i)
        pos_str = f"({intc.position[0]:7.1f}, {intc.position[1]:7.1f})"
        print(f"{intc.id:<16} {role:<12} {pos_str:<24} {intc.fuel:<6.2f} {intc.state}")
    
    print()
    
    # Test bid components with a hypothetical threat
    print("Testing bid components with threat at (5000, 0):\n")
    threat_pos = (5000.0, 0.0)
    
    print(f"{'ID':<16} {'Distance':<10} {'Pint':<8} {'Frem':<8} {'Mpay':<8} {'U':<8}")
    print("-" * 60)
    
    for intc in interceptors:
        comps = intc.get_bid_components(threat_pos)
        print(f"{intc.id:<16} {comps['distance']:<10.1f} {comps['Pint']:<8.3f} "
              f"{comps['Frem']:<8.2f} {comps['Mpay']:<8.2f} {comps['U']:<8.2f}")
    
    print()
    
    # Test state transitions
    print("Testing state transitions:\n")
    
    test_intc = interceptors[0]
    print(f"Interceptor: {test_intc.id}")
    print(f"  Initial state: {test_intc.state}")
    print(f"  Can engage: {test_intc.can_engage()}")
    
    # Assign target
    test_intc.assign_target("threat_1")
    print(f"  After assign_target('threat_1'):")
    print(f"    State: {test_intc.state}")
    print(f"    Assigned threat: {test_intc.assigned_threat_id}")
    print(f"    Can engage: {test_intc.can_engage()}")
    print(f"    Utilization: {test_intc.utilization}")
    
    # Simulate fuel burn
    print(f"\n  Simulating 10 seconds of pursuit...")
    initial_fuel = test_intc.fuel
    for _ in range(100):  # 100 steps * 0.1s = 10s
        test_intc.update(config.TIME_STEP)
    
    fuel_burned = initial_fuel - test_intc.fuel
    print(f"    Fuel burned: {fuel_burned:.3f}")
    print(f"    Fuel remaining: {test_intc.fuel:.3f}")
    
    # Complete intercept
    test_intc.on_intercept_complete()
    print(f"\n  After on_intercept_complete():")
    print(f"    State: {test_intc.state}")
    print(f"    Can engage: {test_intc.can_engage()}")
    
    print()
    
    # Verify other interceptors still idle
    print("Verifying other interceptors remain idle:")
    all_idle = all(intc.is_idle for intc in interceptors[1:])
    print(f"  All others idle: {all_idle}")
    
    print("\n=== Phase 4 Test Complete ===")


if __name__ == "__main__":
    test_formation()
