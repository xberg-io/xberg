package e2e_test

import "encoding/json"

// jsonString converts a value to its JSON string representation.
// Array fields use jsonString instead of fmt.Sprint to preserve structure.
func jsonString(value any) string {
	encoded, err := json.Marshal(value)
	if err != nil {
		return ""
	}
	return string(encoded)
}
