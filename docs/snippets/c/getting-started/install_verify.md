```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const char *version = kreuzberg_version();
    printf("kreuzberg version: %s\n", version ? version : "(unknown)");
    return 0;
}
```
