12

| Table 6. The shortest well-resolved wave ω f and resolving with tolerance error ϵ t = 0.0001. |  |  |  |  | efficiency | e of different schemes |  |  |  |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Schemes/order |  | 4-T |  | 6-T |  | 8-T |  | 10-P |  |
|  | ω f | e | ω f | e | ω f | e | ω f |  | e |
| TDCNCS | 0.392 | 0.1248 | 1.524 | 0.4850 | 1.211 | 0.3855 | 1.293 |  | 0.4114 |
| TDCCCS | 0.405 | 0.1290 | 0.800 | 0.2545 | 1.105 | 0.3518 | 1.493 |  | 0.4753 |
| TDCCCS-CI | 0.405 | 0.1290 | 0.799 | 0.2544 | 1.097 | 0.3490 | 1.362 |  | 0.4336 |
| TDCCS-CI | 0.407 | 0.1294 | 1.994 | 0.6347 | 1.468 | 0.4672 | 1.420 |  | 0.4521 |
| TDCCS-TE | 0.407 | 0.1294 | 0.785 | 0.2497 | 1.689 | 0.5376 | 2.288 |  | 0.7284 |
| TDCCS-LS | 2.792 | 0.8888 | 2.982 | 0.9493 | 3.141 | 0.9998 | 3.044 |  | 0.9690 |
| TDCCS-CI-1 | 0.407 | 0.1294 | 1.994 | 0.6347 | 1.994 | 0.6347 | 1.994 |  | 0.6347 |
| TDCCS-TE-1 | 0.407 | 0.1294 | 0.785 | 0.2497 | 1.659 | 0.5280 | 2.228 |  | 0.7093 |
| TDCCS-LS-1 | 2.792 | 0.8888 | 2.982 | 0.9493 | 3.141 | 0.9998 | 3.141 |  | 0.9998 |
| TDCCS-TE-2 | 0.407 | 0.1294 | 0.854 | 0.2718 | 1.231 | 0.3917 | 1.585 |  | 0.5046 |
| TDCCS-TE-3 | 0.407 | 0.1294 | 0.854 | 0.2718 | 1.234 | 0.3928 | 1.515 |  | 0.4823 |

be expressed as

Lavanya V Salian, Samala Rathan, Debojyoti Ghosh

X<sup>N</sup> a~n~ ˆ ˆ ˆ (5.1) α~F~f~j~−1 + f~j~ + α~F~f~j~+1 = (f~j~+n + f~j~−n) 2

n=0

where f~j~ denotes the given value at point j, and f~j~ˆ denotes the value after filtering. The most natural formulation of the problem involves the transfer function associated with the equation (5.1). P~N~ a~n~cos(nω)

n=0

(5.2) T(ω) = 1 + 2α~F~ cos(ω) To determine the unknown coefficients, we require the exclusion of the highest-frequency mode by imposing the condition T(π) = 0. For adaptability, we maintain α~F~ as an unrestricted parameter. Subsequently, the remaining N equations can be deduced by equating the Taylor series coefficients of the left and right sides. Through this process, equation (5.1) yields a 2N-th order formula within a 2N + 1 point stencil. It is important to note that T(ω) is real, signifying that the filter alters only the amplitude of each wave component without impacting the phase. Here α~F~ is a free parameter that satisfies |α~F~| \< 0.5. The coefficients of a family of 8th-order (F8), 10th-order (F10), and 12th-order filter (F12) are given in the Table 7.

Table 7. Coefficients for the filter formula.

| Scheme | a0 |  | a1 | a2 |  | a3 | a4 |  | a5 | a6 | Order |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  | (93+70α | ) | (7+18α ) | ( − 7+14α | ) | (1 − 2α ) | ( − 1+2α | ) |  |  |  |
| F-8 |  | F | F |  | F | F |  | F | 0 | 0 | 8 |
|  | 128 |  | 16 | 32 |  | 16 | 128 |  |  |  |  |
| F-10 | (193+126α F ) | (105+302α F ) |  | 15( − 1+2α F ) | 45(1 − 2α F ) |  | 5( − 1+2α F ) |  | (1 − 2α F ) | 0 | 10 |
|  | 256 |  | 256 | 64 |  | 512 | 256 |  | 512 |  |  |
|  | (793+462α | ) (99+314α | ) | 495( − 1+2α | ) 55(1 − 2α | ) | 33( − 1+2α | ) | 3(1 − 2α ) | (-1+2α ) |  |
| F-12 |  | F | F |  | F | F |  | F | F | F | 12 |
|  | 1024 |  | 256 | 2048 |  | 512 | 1024 |  | 512 | 2048 |  |
