<!-- snippet:syntax-only -->
```c title="C"
#include <curl/curl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct response_buffer {
    char *data;
    size_t size;
};

static size_t write_callback(void *contents, size_t size, size_t nmemb, void *userp) {
    size_t total = size * nmemb;
    struct response_buffer *buf = (struct response_buffer *)userp;
    char *resized = realloc(buf->data, buf->size + total + 1);
    if (!resized) {
        return 0;
    }
    buf->data = resized;
    memcpy(buf->data + buf->size, contents, total);
    buf->size += total;
    buf->data[buf->size] = '\0';
    return total;
}

int main(void) {
    curl_global_init(CURL_GLOBAL_DEFAULT);

    CURL *curl = curl_easy_init();
    if (!curl) {
        fprintf(stderr, "curl_easy_init failed\n");
        curl_global_cleanup();
        return 1;
    }

    struct response_buffer response = {NULL, 0};

    curl_mime *form = curl_mime_init(curl);
    curl_mimepart *part = curl_mime_addpart(form);
    curl_mime_name(part, "file");
    curl_mime_filedata(part, "document.pdf");

    curl_easy_setopt(curl, CURLOPT_URL, "http://localhost:8000/extract");
    curl_easy_setopt(curl, CURLOPT_MIMEPOST, form);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_callback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);

    CURLcode rc = curl_easy_perform(curl);
    if (rc != CURLE_OK) {
        fprintf(stderr, "request failed: %s\n", curl_easy_strerror(rc));
    } else {
        long status = 0;
        curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
        printf("HTTP %ld\n%s\n", status, response.data ? response.data : "(empty)");
    }

    free(response.data);
    curl_mime_free(form);
    curl_easy_cleanup(curl);
    curl_global_cleanup();
    return rc == CURLE_OK ? 0 : 1;
}
```
