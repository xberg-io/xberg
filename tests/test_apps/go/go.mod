module kreuzberg-test-suite

go 1.25

require (
	github.com/kreuzberg-dev/kreuzberg/packages/go/v4 v4.2.4
	github.com/stretchr/testify v1.11.1
)

require (
	github.com/davecgh/go-spew v1.1.1 // indirect
	github.com/pmezard/go-difflib v1.0.0 // indirect
	gopkg.in/yaml.v3 v3.0.1 // indirect
)

// For testing: use local module instead of GitHub release
replace github.com/kreuzberg-dev/kreuzberg/packages/go/v4 => ../../../packages/go/v4
