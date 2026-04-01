# Fibonacci sequence

The Fibonacci sequence is defined through the recurrence relation $`F_{n} = F_{n - 1} + F_{n - 2}`$. It can also be expressed in *closed form:*

``` math
F_{n} = \left\lfloor {\frac{1}{\sqrt{5}}\phi^{n}} \right\rceil,\quad\phi = \frac{1 + \sqrt{5}}{2}
```

The first 8 numbers of the sequence are:

<div align="center">

|           |           |           |           |           |           |           |           |
|-----------|-----------|-----------|-----------|-----------|-----------|-----------|-----------|
| $`F_{1}`$ | $`F_{2}`$ | $`F_{3}`$ | $`F_{4}`$ | $`F_{5}`$ | $`F_{6}`$ | $`F_{7}`$ | $`F_{8}`$ |
| 1         | 1         | 2         | 3         | 5         | 8         | 13        | 21        |

</div>

<div class="columns-flow" count="2">

<span align="center">[ **Typst Math for Undergrads** ](https://github.com/johanvx/typst-undergradmath)</span>

This is a Typst port of *<span class="box">L A <span class="box">T E X</span></span> Math for Undergrads* by Jim Hefferon. The original version is available at <u><https://gitlab.com/jim.hefferon/undergradmath></u>.

**Meaning of annotations  **

|  |  |
|:---|:---|
| <span class="box">2023-05-22 ❌</span> | This is unavailable. Last check date is 2023-05-22. |

<span id="unavailable"></span>

|  |  |
|:---|:---|
| <span class="box">💦</span> | Get this in a tricky way. Need a simpler method. |

<span id="tricky"></span>

|                                     |                             |
|:------------------------------------|:----------------------------|
| <span class="box">No idea 😕</span> | Don’t know how to get this. |

<span id="noidea"></span>

**Rule One  **Any mathematics at all, even a single character, gets a mathematical setting. Thus, for “the value of $`x`$ is $`7`$” enter `the value of $x$ is $7$`.

**Template  **Your document should contain at least this.

<table>
<tbody>
<tr>
<td></td>
<td><pre><code>-- document body here --</code></pre></td>
</tr>
</tbody>
</table>

**Common constructs  **

<div align="center">

|  |  |
|:---|:---|
| <span class="box">$`x^{2}`$ `x^2`</span> | <span class="box">$`\sqrt{2}`$, $`\sqrt[n]{3}`$ `sqrt(2)`, `root(n, 3)`</span> |
| <span class="box">$`x_{i,j}`$ `x_(i, j)`</span> | <span class="box">$`\frac{2}{3}`$, $`2/3`$ `2 / 3`, `2 \/ 3` or `2 slash 3`</span> |

</div>

**Calligraphic letters  **Use as in `$cal(A)$`.

``` math
\mathcal{ABCDEFGHIJKLMNOPQRSTUVWXYZ}
```

Getting script letters is <a href="#unavailable" class="ref">[unavailable]</a>.

**Greek  **

<div align="center">

|  |  |
|:---|:---|
| <span class="box">$`\alpha`$ `alpha`</span> | <span class="box">$`\xi`$, $`\Xi`$ `xi`, `Xi`</span> |
| <span class="box">$`\beta`$ `beta`</span> | <span class="box">$`ο`$ `omicron`</span> |
| <span class="box">$`\gamma`$, $`\Gamma`$ `gamma`, `Gamma`</span> | <span class="box">$`\pi`$, $`\Pi`$ `pi`, `Pi`</span> |
| <span class="box">$`\delta`$, $`\Delta`$ `delta`, `Delta`</span> | <span class="box">$`\varpi`$ `pi.alt`</span> |
| <span class="box">$`\epsilon`$ `epsilon.alt`</span> | <span class="box">$`\rho`$ `rho`</span> |
| <span class="box">$`\varepsilon`$ `epsilon`</span> | <span class="box">$`\varrho`$ `rho.alt`</span> |
| <span class="box">$`\zeta`$ `zeta`</span> | <span class="box">$`\sigma`$, $`\Sigma`$ `sigma`, `Sigma`</span> |
| <span class="box">$`\eta`$ `eta`</span> | <span class="box">$`\varsigma`$ `\u{03C2}` <a href="#tricky" class="ref">[tricky]</a></span> |
| <span class="box">$`\theta`$, $`\Theta`$ `theta`, `Theta`</span> | <span class="box">$`\tau`$ `tau`</span> |
| <span class="box">$`\vartheta`$ `theta.alt`</span> | <span class="box">$`\upsilon`$, $`\Upsilon`$ `upsilon`, `Upsilon`</span> |
| <span class="box">$`\iota`$ `iota`</span> | <span class="box">$`\phi`$, $`\Phi`$ `phi.alt`, `Phi`</span> |
| <span class="box">$`\kappa`$ $`Κ`$</span> | <span class="box">$`\varphi`$ `phi`</span> |
| <span class="box">$`\lambda`$, $`\Lambda`$ `lambda`, `Lambda`</span> | <span class="box">$`\chi`$ `chi`</span> |
| <span class="box">$`\mu`$ `mu`</span> | <span class="box">$`\psi`$, $`\Psi`$ `psi`, `Psi`</span> |
| <span class="box">$`\nu`$ `nu`</span> | <span class="box">$`\omega`$, $`\Omega`$ `omega`, `Omega`</span> |

</div>

**Sets and logic  **

<div align="center">

|  |  |  |
|:---|:---|:---|
| <span class="box">$`\cup`$ `union`</span> | <span class="box">$`\mathbb{R}`$ `RR`, `bb(R)`</span> | <span class="box">$`\forall`$ `forall`</span> |
| <span class="box">$`\cap`$ `sect`</span> | <span class="box">$`\mathbb{Z}`$ `ZZ`, `bb(Z)`</span> | <span class="box">$`\exists`$ `exists`</span> |
| <span class="box">$`\subset`$ `subset`</span> | <span class="box">$`\mathbb{Q}`$ `QQ`, `bb(Q)`</span> | <span class="box">$`\neg`$ `not`</span> |
| <span class="box">$`\subseteq`$ `subset.eq`</span> | <span class="box">$`\mathbb{N}`$ `NN`, `bb(N)`</span> | <span class="box">$`\vee`$ `or`</span> |
| <span class="box">$`\supset`$ `supset`</span> | <span class="box">$`\mathbb{C}`$ `CC`, `bb(C)`</span> | <span class="box">$`\land`$ `and`</span> |
| <span class="box">$`\supseteq`$ `supset.eq`</span> | <span class="box">$`\varnothing`$ `diameter`</span> | <span class="box">$`\vdash`$ `tack.r`</span> |
| <span class="box">$`\in`$ `in`</span> | <span class="box">$`\varnothing`$ `nothing`</span> | <span class="box">$`\models`$ `models`</span> |
| <span class="box">$`\notin`$ `in.not`</span> | <span class="box">$`א`$ `alef`</span> | <span class="box">$`\smallsetminus`$ `without`</span> |

</div>

Negate an operator, as in $`⊄`$, with `subset.not`. Get the set complement $`A^{\mathsf{c}}`$ with `A^(sans(c))` (or $`A^{\complement}`$ with `A^(complement)`, or $`\overline{A}`$ with `overline(A)`).

Remark  
Using `diameter` for `\varnothing` may cause some confusion. However, <span class="box">L A <span class="box">T E X</span></span> also uses $`\varnothing`$ (`\u{2300}`) instead of $`\varnothing`$ (`\u{2205}`), see <u>[newcm $`§`$<!-- -->13.3](https://mirrors.sustech.edu.cn/CTAN/fonts/newcomputermodern/doc/newcm-doc.pdf)</u>. Another solution is to use `text(font: "Fira Sans", nothing)`, but the resultant glyph $`\varnothing`$ is subtly different from the widely used one. Ultimately, the choice is always **your decision**.

**Decorations  **

<div align="center">

|  |  |  |
|:---|:---|:---|
| <span class="box">$`f'`$ `f'`, `f prime`</span> | <span class="box">$`\dot{a}`$ `dot(a)`</span> | <span class="box">$`\widetilde{a}`$ `tilde(a)`</span> |
| <span class="box">$`f''`$ `f prime.double`</span> | <span class="box">$`\ddot{a}`$ `diaer(a)`</span> | <span class="box">$`\overline{a}`$ `macron(a)`</span> |
| <span class="box">$`\Sigma^{\ast}`$ `Sigma^*`</span> | <span class="box">$`\hat{a}`$ `hat(a)`</span> | <span class="box">$`\overset{\rightarrow}{a}`$ `arrow(a)`</span> |

</div>

If the decorated letter is $`i`$ or $`j`$ then some decorations need `\u{1D6A4}` <a href="#tricky" class="ref">[tricky]</a> and `\u{1D6A5}` <a href="#tricky" class="ref">[tricky]</a>, as in $`\overset{\rightarrow}{\imath}`$ with `arrow(\u{1D6A4})`. Some authors use boldface for vectors: `bold(x)`.

Entering `overline(x + y)` produces $`\overline{x + y}`$, and `hat(x + y)` gives $`\hat{x + y}`$. Comment on an expression as here (there is also `overbrace(..)`).

<span align="center"><span class="box">$`\underset{|A|}{\underbrace{x + y}}`$ `underbrace(x + y, |A|)`</span></span>

**Dots  **Use low dots in a list $`\left\{ 0,1,2,\ldots \right\}`$, entered as `{0, 1, 2, ...}`. Use centered dots in a sum or product $`1 + \cdots + 100`$, entered as `1 + dots.h.c + 100`. You can also get vertical dots `dots.v`, diagonal dots `dots.down` and anti-diagonal dots `dots.up`.

**Roman names  **Just type them!

<div align="center">

|  |  |  |
|:---|:---|:---|
| <span class="box">$`\sin`$ `sin`</span> | <span class="box">$`\sinh`$ `sinh`</span> | <span class="box">$`\arcsin`$ `arcsin`</span> |
| <span class="box">$`\cos`$ `cos`</span> | <span class="box">$`\cosh`$ `cosh`</span> | <span class="box">$`\arccos`$ `arccos`</span> |
| <span class="box">$`\tan`$ `tan`</span> | <span class="box">$`\tanh`$ `tanh`</span> | <span class="box">$`\arctan`$ `arctan`</span> |
| <span class="box">$`\sec`$ `sec`</span> | <span class="box">$`\coth`$ `coth`</span> | <span class="box">$`\min`$ `min`</span> |
| <span class="box">$`\csc`$ `csc`</span> | <span class="box">$`\det`$ `det`</span> | <span class="box">$`\max`$ `max`</span> |
| <span class="box">$`\cot`$ `cot`</span> | <span class="box">$`\dim`$ `dim`</span> | <span class="box">$`\inf`$ `inf`</span> |
| <span class="box">$`\exp`$ `exp`</span> | <span class="box">$`\ker`$ `ker`</span> | <span class="box">$`\sup`$ `sup`</span> |
| <span class="box">$`\log`$ `log`</span> | <span class="box">$`\deg`$ `deg`</span> | <span class="box">$`\liminf`$ `liminf`</span> |
| <span class="box">$`\ln`$ `ln`</span> | <span class="box">$`\arg`$ `arg`</span> | <span class="box">$`\limsup`$ `limsup`</span> |
| <span class="box">$`\lg`$ `lg`</span> | <span class="box">$`\gcd`$ `gcd`</span> | <span class="box">$`\lim`$ `lim`</span> |

</div>

**Other symbols  **

<div align="center">

|  |  |  |
|:---|:---|:---|
| <span class="box">$`<`$ `<`, `lt`</span> | <span class="box">$`\angle`$ `angle`</span> | <span class="box">$`\cdot`$ `dot`</span> |
| <span class="box">$`\leq`$ `<=`, `lt.eq`</span> | <span class="box">$`\measuredangle`$ `angle.arc`</span> | <span class="box">$`\pm`$ `plus.minus`</span> |
| <span class="box">$`>`$ `>`, `gt`</span> | <span class="box">$`\ell`$ `ell`</span> | <span class="box">$`\mp`$ `minus.plus`</span> |
| <span class="box">$`\geq`$ `>=`, `gt.eq`</span> | <span class="box">$`\parallel`$ `parallel`</span> | <span class="box">$`\times`$ `times`</span> |
| <span class="box">$`\neq`$ `!=`, `eq.not`</span> | <span class="box">$`45{^\circ}`$ `45 degree`</span> | <span class="box">$`\div`$ `div`</span> |
| <span class="box">$`\ll`$ `<<`, `lt.double`</span> | <span class="box">$`\cong`$ `tilde.equiv`</span> | <span class="box">$`\ast`$ `*`, `ast`</span> |
| <span class="box">$`\gg`$ `>>`, `gt.double`</span> | <span class="box">$`\ncong`$ `tilde.equiv.not`</span> | <span class="box">$`\mid`$ `divides`</span> |
| <span class="box">$`\approx`$ `approx`</span> | <span class="box">$`\sim`$ `tilde`</span> | <span class="box">$`\nmid`$ `divides.not`</span> |
| <span class="box">$`\asymp`$ `\u{224D}` <a href="#tricky" class="ref">[tricky]</a></span> | <span class="box">$`\simeq`$ `tilde.eq`</span> | <span class="box">$`n!`$ `n!`</span> |
| <span class="box">$`\equiv`$ `equiv`</span> | <span class="box">$`\nsim`$ `tilde.not`</span> | <span class="box">$`\partial`$ `diff`</span> |
| <span class="box">$`\prec`$ `prec`</span> | <span class="box">$`\oplus`$ `plus.circle`</span> | <span class="box">$`\nabla`$ `nabla`</span> |
| <span class="box">$`\preceq`$ `prec.eq`</span> | <span class="box">$`\ominus`$ `minus.circle`</span> | <span class="box">$`ħ`$ `planck.reduce`</span> |
| <span class="box">$`\succ`$ `succ`</span> | <span class="box">$`\odot`$ `dot.circle`</span> | <span class="box">$`\circ`$ `circle.stroked.tiny`</span> |
| <span class="box">$`\succeq`$ `succ.eq`</span> | <span class="box">$`\otimes`$ `times.circle`</span> | <span class="box">$`\star`$ `star`</span> |
| <span class="box">$`\propto`$ `prop`</span> | <span class="box">$`\oslash`$ `\u{2298}` <a href="#tricky" class="ref">[tricky]</a></span> | <span class="box">$`\sqrt{}`$ `sqrt("")`</span> |
| <span class="box">$`\doteq`$ `\u{2250}` <a href="#tricky" class="ref">[tricky]</a></span> | <span class="box">$`\upharpoonright`$ `harpoon.tr`</span> | <span class="box">$`✓`$ `checkmark`</span> |

</div>

Use `a divides b` for the divides relation, $`a \mid b`$, and `a divides.not b` for the negation, $`a \nmid b`$. Use `|` to get set builder notation $`\left\{ a \in S~|~a\text{ is odd} \right\}`$ with `{a in S | a "is odd"}`.

**Arrows  **

<div align="center">

|  |  |
|:---|:---|
| <span class="box">$`\rightarrow`$ `->`, `arrow.r`</span> | <span class="box">$`\mapsto`$ `|->`, `arrow.r.bar`</span> |
| <span class="box">$`\nrightarrow`$ `arrow.r.not`</span> | <span class="box">$`\longmapsto`$ `arrow.r.long.bar`</span> |
| <span class="box">$`\longrightarrow`$ `arrow.r.long`</span> | <span class="box">$`\leftarrow`$ `<-`, `arrow.l`</span> |
| <span class="box">$`\Rightarrow`$ `=>`, `arrow.r.double`</span> | <span class="box">$`\longleftrightarrow`$ `<-->`, `arrow.l.r.long`</span> |
| <span class="box">$`\nRightarrow`$ `arrow.r.double.not`</span> | <span class="box">$`\downarrow`$ `arrow.b`</span> |
| <span class="box">$`\Longrightarrow`$ `arrow.r.double.long`</span> | <span class="box">$`\uparrow`$ `arrow.t`</span> |
| <span class="box">$`\rightsquigarrow`$ `arrow.squiggly`</span> | <span class="box">$`\updownarrow`$ `arrow.t.b`</span> |

</div>

The right arrows in the first column have matching left arrows, such as `arrow.l.not`, and there are some other matches for down arrows, etc.

**Variable-sized operators  **The summation $`\sum_{j = 0}^{3}j^{2}`$ `sum_(j = 0)^3 j^2` and the integral $`\int_{x = 0}^{3}x^{2}dx`$ `integral_(x = 0)^3 x^2 dif x` expand when displayed.

``` math
\sum_{j = 0}^{3}j^{2}\qquad\int_{x = 0}^{3}x^{2}dx
```

These do the same.

<div align="center">

|  |  |  |
|:---|:---|:---|
| <span class="box">$`\int`$ `integral`</span> | <span class="box">$`\iiint`$ `integral.triple`</span> | <span class="box">$`\bigcup`$ `union.big`</span> |
| <span class="box">$`\iint`$ `integral.double`</span> | <span class="box">$`\oint`$ `integral.cont`</span> | <span class="box">$`\bigcap`$ `sect.big`</span> |

</div>

**Fences  **

<div align="center">

|  |  |  |
|:---|:---|:---|
| <span class="box">$`()`$ `()`</span> | <span class="box">$`\langle\rangle`$ `angle.l angle.r`</span> | <span class="box">$`\left| {} \right|`$ `abs("")`</span> |
| <span class="box">$`\lbrack\rbrack`$ `[]`</span> | <span class="box">$`\left\lfloor {} \right\rfloor`$ `floor("")`</span> | <span class="box">$`\left\| {} \right\|`$ `norm("")`</span> |
| <span class="box">$`\left\{ \right\}`$ `{}`</span> | <span class="box">$`\left\lceil {} \right\rceil`$ `ceil("")`</span> |  |

</div>

Fix the size with the `lr` function.

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display">$$\left. \left\lbrack \sum_{k = 0}^{n}e^{k^{2}} \right\rbrack \right.$$</span></p></td>
<td style="text-align: left;"><pre><code>lr([sum_(k = 0)^n e^(k^2)], size: #50%)</code></pre></td>
</tr>
</tbody>
</table>

</div>

To have them grow with the enclosed formula, also use the `lr` function.

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display">⟨<em>i</em>, 2<sup>2<sup><em>i</em></sup></sup>⟩</span></p></td>
<td style="text-align: left;"><pre><code>lr(angle.l i, 2^(2^i) angle.r)</code></pre></td>
</tr>
</tbody>
</table>

</div>

Fences scale by default if entered directly as codepoints, and don’t scale automatically if entered as symbol notation.

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display">$$\left( \frac{1}{n^{\alpha}} \right)$$</span></p></td>
<td style="text-align: left;"><pre><code>(1 / n^(alpha))</code></pre></td>
</tr>
<tr>
<td style="text-align: left;"><p><span class="math display">$$(\frac{1}{n^{\alpha}})$$</span></p></td>
<td style="text-align: left;"><pre><code>paren.l 1 / n^(alpha) paren.r</code></pre></td>
</tr>
</tbody>
</table>

</div>

The `lr` function also allows to scale unmatched delimiters and one-side fences.

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display">$$\left. \frac{df}{dx} \right|_{x_{0}}$$</span></p></td>
<td style="text-align: left;"><pre><code>lr(frac(dif f, dif x) |)_(x_0)</code></pre></td>
</tr>
</tbody>
</table>

</div>

**Arrays, Matrices  **Get a matrix with the `mat` function. You can pass an array to it.

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display">$$\begin{pmatrix}
a &amp; b \\
c &amp; d
\end{pmatrix}$$</span></p></td>
<td style="text-align: left;"><pre><code>$ mat(a, b; c, d) $</code></pre></td>
</tr>
</tbody>
</table>

</div>

In Typst, <u>[array](https://typst.app/docs/reference/typst/array)</u> is a sequence of values, while in <span class="box">L A <span class="box">T E X</span></span>, array is a matrix without fences, which is `$mat(delim: #none, ..)$` in Typst.

For the determinant use `|A|`, text operator $`\det`$ `det` or `mat(delim: "|", ..)`.

Definition by cases can be easily obtained with the `cases` function.

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display">$$f_{n} = \begin{cases}
a &amp; \text{if }n = 0 \\
r \cdot f_{n - 1} &amp; \text{else }
\end{cases}$$</span></p></td>
<td style="text-align: left;"><pre><code>$ f_n = cases(
  a &amp;&quot;if&quot; n = 0,
  r dot f_(n - 1) &amp;&quot;else&quot;
) $</code></pre></td>
</tr>
</tbody>
</table>

</div>

**Spacing in mathematics  **Improve $`\sqrt{2}x`$ to $`\sqrt{2}\, x`$ with a thin space, as in `sqrt(2) thin x`. Slightly wider are `medium` and `thick` (the three are in ratio $`3:4:5`$). Bigger space is `quad` for $`\rightarrow \quad \leftarrow`$, which is useful between parts of a display. Get arbitrary space with the `h` function. For example, use `#h(2em)` for `\qquad` in <span class="box">L A <span class="box">T E X</span></span> and `#h(-0.1667em)` for `\!`.

**Displayed equations  **Display equations in a block level using `$ ... $` with at least one space separating the math content and the `$`.

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display"><em>S</em> = <em>k</em> ⋅ lg <em>W</em></span></p></td>
<td style="text-align: left;"><pre><code>$ S = k dot lg W $</code></pre></td>
</tr>
</tbody>
</table>

</div>

You can break into multiple lines.

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display">$$\begin{array}{r}
\sin(x) = x - \frac{x^{3}}{3!} \\
 + \frac{x^{5}}{5!} - \cdots
\end{array}$$</span></p></td>
<td style="text-align: left;"><pre><code>$ sin(x) = x - x^3 / 3! \
    + x^5 / 5! - dots.h.c $</code></pre></td>
</tr>
</tbody>
</table>

</div>

Align equations using `&`

<div align="center">

<table>
<tbody>
<tr>
<td style="text-align: left;"><p><span class="math display">$$\begin{aligned}
\nabla \cdot \mathbf{D} &amp; = \rho \\
\nabla \cdot \mathbf{B} &amp; = 0
\end{aligned}$$</span></p></td>
<td style="text-align: left;"><pre><code>$ nabla dot bold(D) &amp;= rho \
  nabla dot bold(B) &amp;= 0 $</code></pre></td>
</tr>
</tbody>
</table>

</div>

(the left or right side of an alignment can be empty). Get a numbered version by `#set math.equation(numbering: ..)`.

**Calculus examples  **The last three here are display style.

<div align="center">

<table>
<tbody>
<tr>
<td><p><span class="math inline"><em>f</em> : ℝ → ℝ</span></p></td>
<td><pre><code>f: RR -&gt; RR</code></pre></td>
</tr>
<tr>
<td><p><span class="math inline">9.8  m/s<sup>2</sup></span></p></td>
<td><p><code>"9.8" "m/s"^2</code> <a href="#tricky" class="ref">[tricky]</a></p></td>
</tr>
<tr>
<td><p><span class="math display">$$\lim\limits_{h \rightarrow 0}\frac{f(x + h) - f(x)}{h}$$</span></p></td>
<td><pre><code>lim_(h -&gt; 0) (f(x + h) - f(x)) / h</code></pre></td>
</tr>
<tr>
<td><p><span class="math display">∫<em>x</em><sup>2</sup><em>d</em><em>x</em> = <em>x</em><sup>3</sup>/3 + <em>C</em></span></p></td>
<td><pre><code>integral x^2 dif x = x^3 \/ 3 + C</code></pre></td>
</tr>
<tr>
<td><p><span class="math display">$$\nabla = \mathbf{i}\frac{d}{dx} + \mathbf{j}\frac{d}{dy} + \mathbf{k}\frac{d}{dz}$$</span></p></td>
<td><pre><code>nabla = bold(i) dif / (dif x) + bold(j) dif / (dif y) + bold(k) dif / (dif z)</code></pre></td>
</tr>
</tbody>
</table>

</div>

**Discrete mathematics examples  **For modulo, there is a symbol $`\equiv`$ from `equiv` and a text operator $`\operatorname{mod}`$ from `mod`.

For combinations the binomial symbol $`\binom{n}{k}`$ is from `binom(n, k)`. This resizes to be bigger in a display.

For permutations use $`n^{\underline{r}}`$ from `n^(underline(r))` (some authors use $`P(n,r)`$, or $`{}_{n}P_{r}`$ from `""_n P_r`).

**Statistics examples  **

<div align="center">

<table>
<tbody>
<tr>
<td><p><span class="math inline">$\sigma^{2} = \sqrt{{\sum(x_{i} - \mu)}^{2}/N}$</span></p></td>
<td><pre><code>sigma^2 = sqrt(sum(x_i - mu)^2 \/ N)</code></pre></td>
</tr>
<tr>
<td><p><span class="math inline"><em>E</em>(<em>X</em>) = <em>μ</em><sub><em>X</em></sub> = ∑(<em>x</em><sub><em>i</em></sub> − <em>P</em>(<em>x</em><sub><em>i</em></sub>))</span></p></td>
<td><pre><code>E(X) = mu_X = sum(x_i - P(x_i))</code></pre></td>
</tr>
</tbody>
</table>

</div>

The probability density of the normal distribution

``` math
\frac{1}{\sqrt{2\sigma^{2}\pi}}e^{- \frac{(x - \mu)^{2}}{2\sigma^{2}}}
```

comes from this.

<table>
<tbody>
<tr>
<td></td>
<td><pre><code>1 / sqrt(2 sigma^2 pi)
  e^(- (x - mu)^2 / (2 sigma^2))</code></pre></td>
</tr>
</tbody>
</table>

**For more  **See also the Typst Documentation at <u><https://typst.app/docs></u>.

------------------------------------------------------------------------

johanvx (<u><https://github.com/johanvx></u>)    2023-05-22

</div>
