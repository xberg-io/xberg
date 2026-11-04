**3.8 Overlapping Template Matching Test**

This test rejects sequences which show too many or too few occurrences of *m* -runs of ones, but can be easily modified to detect irregular occurrences of any periodic pattern *B* .

To implement this test, parameters *M* and *N* are determined so that *n* = *MN* , i.e., the original string is partitioned into *N* blocks, each of length *M* .

˜ ˜

Let *W* = *W*

( *m* , *n* ) be the number of (possibly overlapping) runs of ones of length *m* in the jth block.

*j j*

The asymptotic distribution of *Wj* is the compound Poisson distribution (the so-called Pòlya-Aeppli law, see[1]):

λ

*t* 1

# ( e − ) E exp{   (5)

*tW* ˜ *j* } → exp  2 − *et*    when ( *M* − *m* + 1)2− *m* →λ \> 0 ( *t* is a real variable).

The corresponding probabilities can be expressed in terms of the confluent hypergeometric function Φ = *F* . If *U* denotes a random variable with the compound Poisson asymptotic distribution, then for u ≥

1 1

1 with η = λ/2.

*u*

  *l e* −η *u* −1 η ηe

−2η

*P* ( *U* = *u* ) =

∑  = Φ(

*u* +1,2,η). 2

*u*

 *l* −1 *l* \! 2

*u l* =1

For example, *P* ( *U* = 0) = *e* −η, η *P* ( *U* = 1) = *e*

−η, 2 ηe−η

*P* ( *U* = 2) =

## [η + 2]

, 8 

2

ηe 

−η η *P* (

*U* = 3) =  + η + 1, 8  6  

3

ηe 

−η η η2 3η *P* (

*U* = 4) =  + + + 1.

following probabilities:

16 24 2 2  More accurate, but also more complicated, expressions are given in [3]. These expressions yield the

|  |  |
| --- | --- |
| π 0 | 0.364091 |
| π 1 | 0.185659 |
| π2 | 0.139381 |
| π 3 | 0.100571 |
| π4 | 0.0704323 |
| π 5 | 0.139865 |

2 4
