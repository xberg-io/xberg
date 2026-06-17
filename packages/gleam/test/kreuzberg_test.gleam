import gleeunit
import gleeunit/should

pub fn main() {
  gleeunit.main()
}

/// Smoke test: confirms the test runner is wired up. Replace with real tests
/// that import the generated module and exercise its functions.
pub fn smoke_test() {
  should.equal(1, 1)
}
