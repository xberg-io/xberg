```go title="Go"
package main

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/xberg-ffi
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -lxberg_ffi
#include "../../../crates/xberg-ffi/xberg.h"
#include <stdlib.h>
*/
import "C"
import (
	"log"
	"unsafe"

	"github.com/xberg-io/xberg/packages/go"
)

//export customValidator
func customValidator(resultJSON *C.char) *C.char {
	// Inspect resultJSON, return error message or NULL
	return nil
}

func main() {
	if err := xberg.RegisterValidator("go-validator", 50, (C.ValidatorCallback)(C.customValidator)); err != nil {
		log.Fatalf("register validator failed: %v", err)
	}

	result, err := xberg.ExtractSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Printf("Content length: %d", len(result.Content))
}
```
