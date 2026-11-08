Central compact schemes for third derivatives 11

In Figure 4, the changes in the relative modified wavenumber factor, denoted as R~ω~<sup>′′′</sup>, versus wavenumbers for TDCCS-TE and TDCCS-LS are shown. In comparison to TDCCS-TE, TDCCS-LS notably expands the range of wavenumbers while maintaining the same order of truncation error. The eighth-order tridiagonal TDCCS-LS even exhibits comparable accuracy to the tenth-order pentadiagonal TDCCS-TE. The bandwidth resolving efficiency [10] is a quantitative indicator of spectral resolution. The resolving efficiency of the FD scheme, with a specified error tolerance, is defined as ω~f~ e = , π where ω~f~ is the shortest well-resolved wave component satisfying

′′′

ω (ω) − ω<sup>3</sup> ≤ ϵ ,~t~

3

ω and ϵ~t~ represents the error tolerance threshold. This threshold remains constant when comparing various compact FD schemes. Tables (5) and (6) present the values of the bandwidth resolving efficiency (e) for various compact finite difference (FD) schemes, with error tolerances set at ϵ~t~ = 0.001 and ϵ~t~ = 0.0001, respectively. Tables (5) and (6) highlight that TDCCS exhibits the highest resolving efficiency, effectively capturing a broader range of wavenumbers under the same error tolerance. Specifically, for ϵ~t~ = 0.001, the resolving efficiency of an eighth-order TDCNCS is 0.5018, while that of TDCCS-TE and TDCCS-LS is 0.7828 and 0.9998, respectively. Similarly, for ϵ~t~ = 0.0001, the resolving efficiency of an eighth-order TDCNCS is

0.3855, whereas that of TDCCS-TE and TDCCS-LS is 0.5376 and 0.9998, respectively. Additionally, when compared to other potential combinations such as TDCCS-TE-1, TDCCS-TE-2, and TDCCS-TE-3, TDCCS-

TE exhibits favorable resolving efficiency. Therefore, we adopt the TDCCS-TE scheme in this paper.

| Table 5. The with tolerance | shortest well-resolved wave ω f and resolving error ϵ t = 0.001. |  |  |  | efficiency | e of different schemes |  |  |  |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Schemes/order |  | 4-T |  | 6-T |  | 8-T |  | 10-P |  |
|  | ω f | e | ω f | e | ω f | e | ω f |  | e |
| TDCNCS | 0.693 | 0.2205 | 1.735 | 0.5523 | 1.576 | 0.5018 | 1.635 |  | 0.5205 |
| TDCCCS | 0.716 | 0.2278 | 1.164 | 0.3705 | 1.468 | 0.4672 | 1.845 |  | 0.5874 |
| TDCCCS-CI | 0.715 | 0.2277 | 1.162 | 0.3699 | 1.445 | 0.4600 | 1.682 |  | 0.5354 |
| TDCCS-CI | 0.722 | 0.2297 | 2.037 | 0.6483 | 1.769 | 0.5631 | 1.748 |  | 0.5565 |
| TDCCS-TE | 0.722 | 0.2297 | 1.386 | 0.4411 | 2.459 | 0.7828 | 2.998 |  | 0.9542 |
| TDCCS-LS | 2.795 | 0.8898 | 3.141 | 0.9998 | 3.141 | 0.9998 | 3.141 |  | 0.9998 |
| TDCCS-CI-1 | 0.722 | 0.2297 | 2.037 | 0.6483 | 2.037 | 0.6483 | 2.037 |  | 0.6483 |
| TDCCS-TE-1 | 0.722 | 0.2297 | 1.386 | 0.4411 | 2.412 | 0.7679 | 2.928 |  | 0.9321 |
| TDCCS-LS-1 | 2.795 | 0.8898 | 3.141 | 0.9998 | 3.141 | 0.9998 | 3.141 |  | 0.9998 |
| TDCCS-TE-2 | 0.722 | 0.2297 | 1.244 | 0.3959 | 1.612 | 0.5131 | 2.044 |  | 0.6506 |
| TDCCS-TE-3 | 0.722 | 0.2297 | 1.244 | 0.3959 | 1.618 | 0.5152 | 1.919 |  | 0.6110 |

5. Low-pass spatial filter

With high-order finite difference schemes, it is necessary to artificially mitigate all spurious waves while preserving the accuracy and resolution of the computed solution. This can be performed by regularizing the computed solution via a high-order low-pass spatial filter (LPSF) [10]. A tridiagonal filter of high order can
