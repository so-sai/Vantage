// Test file with epistemic claims
use vantage_core::cognition::*;

fn main() {
    println!("Test function");

    // @epistemic:test-uuid-1
    fn test_function() {
        println!("This is a test function");
    }

    // @epistemic:test-uuid-2
    struct TestStruct {
        field: i32,
    }

    impl TestStruct {
        fn new() -> Self {
            TestStruct { field: 42 }
        }
    }

    // @epistemic:test-uuid-3
    const TEST_CONST: i32 = 123;

    // @epistemic:orphan-uuid
    // This should create an orphan tag
}
