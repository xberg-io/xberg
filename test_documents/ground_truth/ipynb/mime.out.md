

``` python
from __future__ import annotations

from dataclasses import dataclass
```

``` python
```

    7.29.0

Supported IPython display formatters:

``` python
ip = get_ipython()
for mime in ip.display_formatter.formatters:
    pass
```

    text/plain
    text/html
    text/markdown
    image/svg+xml
    image/png
    application/pdf
    image/jpeg
    text/latex
    application/json
    application/javascript

Let's write a simple class that will output different mime:

``` python
@dataclass
class Mime:
    math: str

    def _repr_mimebundle_(
        self,
        include: Container[str] | None = None,
        exclude: Container[str] | None = None,
        **kwargs,
    ) -> dict[str, str]:
        string = self.math
        data = {
            "text/plain": string,
            "text/html": (latex := f"\\[{string}\\]"),
            "text/markdown": f"$${string}$$",
            # "image/svg+xml":,
            # "image/png":,
            # "application/pdf":,
            # "image/jpeg":,
            "text/latex": latex,
            # "application/json":,
            # "application/javascript":,
        }
        if include:
            data =
        if exclude:
            data =
        return data
```

``` python
mime = Mime("E = mc^2")
```

``` python
mime
```

    E = mc^2

Note that \#7561 made ipynb reader aware of this, and \#7563 made ipynb writer aware of this.
