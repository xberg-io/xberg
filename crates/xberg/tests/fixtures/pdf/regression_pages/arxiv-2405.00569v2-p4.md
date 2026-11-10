4 Lavanya V Salian, Samala Rathan, Debojyoti Ghosh

We have extended the expression (2.3) to achieve up to tenth-order accuracy, denoted as the third derivative cell-node compact scheme (TDCNCS). The general form is given by (2.4) f~j~+2 − 2f~j~+1 + 2f~j~−1 − f~j~−2 f~j~+3 − 3f~j~+1 + 3f~j~−1 − f~j~−3

′′′ ′′′ ′′′ ′′′ ′′′

βf~j~−2 + αf~j~−1 + f~j~ + αf~j~+1 + βf~j~+2 = a + b 2h<sup>3</sup> 8h<sup>3</sup> f~j~+4 − 4f~j~+1 + 4f~j~−1 − f~j~−4 + c . 20h<sup>3</sup> The third derivative cell-centered compact scheme (TDCCCS) is given by (2.5) fj+3 − 3fj+1 + 3fj−1 − fj−3 fj+5 − 5fj+1 + 5fj−1 − fj−5

′′′ ′′′ ′′′ ′′′ ′′′

2 2 2 2 2 2 2 2 βf~j~−2 + αf~j~−1 + f~j~ + αf~j~+1 + βf~j~+2 = a + b h<sup>3</sup> 5h<sup>3</sup> fj+72 − 7fj+21 + 7fj−12 − fj−27 + c . 14h<sup>3</sup>

| Scheme | Table 1. The coefficients of TDCNCS schemes. a | b | c | α | β | Order |
| --- | --- | --- | --- | --- | --- | --- |
| TDCNCS-E2 | 1 | 0 | 0 | 0 | 0 | 2 |
| TDCNCS-E4 | 2 | -1 | 0 | 0 | 0 | 4 |
| TDCNCS-E6 | 169 | -12 | 7 | 0 | 0 | 6 |
|  | 60 | 5 | 12 |  |  |  |
| TDCNCS-T4 | 2 | 0 | 0 | 1 | 0 | 4 |
|  |  |  |  | 2 |  |  |
| TDCNCS-T6 | 2 | -1 | 0 | 7 | 0 | 6 |
|  |  | 8 |  | 16 |  |  |
| TDCNCS-T8 | 2367 | -167 | 1 | -205 | 0 | 8 |
|  | 1180 | 1180 | 236 | 472 |  |  |
| TDCNCS-P6 | 40 | 0 | 0 | 4 | 1 | 6 |
|  | 21 |  |  | 9 | 126 |  |
| TDCNCS-P8 | 160 | -5 | 0 | 147 | -1 | 8 |
|  | 83 | 166 |  | 332 | 166 |  |
| TDCNCS-P10 | 18221 | -1846 | 5 | 799 | -557 | 10 |
|  | 5478 | 913 | 66 | 2739 | 5478 |  |

| Scheme | a Table | 2. The coefficients b | of c | TDCCCS schemes. α | β | Order |
| --- | --- | --- | --- | --- | --- | --- |
| TDCCCS-E2 | 1 | 0 | 0 | 0 | 0 | 2 |
| TDCCCS-E4 | 13 | -5 | 0 | 0 | 0 | 4 |
|  | 8 | 8 |  |  |  |  |
| TDCCCS-E6 | 1299 | -499 | 259 | 0 | 0 | 6 |
|  | 640 | 384 | 960 |  |  |  |
| TDCCCS-T4 | 4 | 0 | 0 | 1 | 0 | 4 |
|  | 3 |  |  | 6 |  |  |
| TDCCCS-T6 | 205 | 35 | 0 | 37 | 0 | 6 |
|  | 166 | 166 |  | 166 |  |  |
| TDCCCS-T8 | 1058279 | 96627 | -24787 | 3229 | 0 | 8 |
|  | 975200 | 195040 | 487600 | 12190 |  |  |
| TDCCCS-P6 | 320 | 0 | 0 | 134 | -7 | 6 |
|  | 233 |  |  | 699 | 1398 |  |
| TDCCCS-P8 | 49720 | 91400 | 0 | 28838 | 3541 | 8 |
|  | 79903 | 79903 |  | 79903 | 159806 |  |
| TDCCCS-P10 | 55463611 | 677644345 | 6301771 | 93443398 | 15505921 | 10 |
|  | 150617762 | 451853286 | 225926643 | 225926643 | 451853286 |  |
