**3.8 Overlapping Template Matching Test**

This test rejects sequences which show too many or too few occurrences of *m* -runs of ones, but can be easily modified to detect irregular occurrences of any periodic pattern *B* .

To implement this test, parameters *M* and *N* are determined so that *n* = *MN* , i.e., the original string is partitioned into *N* blocks, each of length *M* .

˜ ˜

Let *W* = *W*

(<sup>*m*</sup> , *n*) <sup>be the number of (possibly overlapping) runs of ones of length</sup> *m* in the jth block.

*j j*

The asymptotic distribution of *W*~*j*~ is the compound Poisson distribution (the so-called Pòlya-Aeppli law, see[1]):

λ(<sup>*e*</sup>*t* −1)<sup></sup> ~~ *E* exp ˜ {<sup>*tW*</sup>~*j*~ } <sup>→</sup> exp  (5)

*t*

 2 − *e*    when (<sup>*M*</sup> − *m* + 1)<sup>2</sup>− *m* → λ \> 0 ( *t* is a real variable).

The corresponding probabilities can be expressed in terms of the confluent hypergeometric function Φ = *F* . If *U* denotes a random variable with the compound Poisson asymptotic distribution, then for u ≥ 1 1 1 with η = λ/2.

−η *u*

 

*l* −2η

*e u* −1 η ηe *P* (<sup>*U*</sup> = *u*)<sup>=</sup>

∑~~

 = Φ(<sup>*u*</sup> +1,2,η)<sup>.</sup>

*u*

2<sup>*u*</sup>  *l* −1 *l* \! 2

*l* =1

For example, *P* ( <sup>*U*</sup> = 0)<sup>=</sup> *e* −η, η

−η

*P* (<sup>*U*</sup> = 1)<sup>=</sup> *e* , 2

−η

ηe *P* ( <sup>*U*</sup> = 2)<sup>=</sup> [<sup>η</sup> + 2]<sup>,</sup> 8

−η 2

ηe η  *P* ( <sup>*U*</sup> = 3)<sup>=</sup>  + η + 1, 8  6 

−η 3

ηe η η<sup>2</sup> 3η  *P* (<sup>*U*</sup> = 4)<sup>=</sup>  + + + 1.

following probabilities:

16 24 2 2  More accurate, but also more complicated, expressions are given in [3]. These expressions yield the

| π ~0~ | 0.364091 |
| --- | --- |
| π 1 | 0.185659 |
| π2 | 0.139381 |
| π 3 | 0.100571 |
| π4 | 0.0704323 |
| π 5 | 0.139865 |
