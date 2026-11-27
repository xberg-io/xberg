**3.8 Overlapping Template Matching Test**

This test rejects sequences which show too many or too few occurrences of *m* -runs of ones, but can be easily modified to detect irregular occurrences of any periodic pattern *B* .

To implement this test, parameters *M* and *N* are determined so that *n* = *MN* , i.e., the original string is partitioned into *N* blocks, each of length *M* .

Let *W* ˜ = *W* ˜ *m* in the jth block.

*j j* (<sup>*m*</sup> , *n*) <sup>be the number of (possibly overlapping) runs of ones of length</sup>

The asymptotic distribution of *W*~*j*~ is the compound Poisson distribution (the so-called Pòlya-Aeppli law, see[1]):



*t*

λ(<sup>*e*</sup> −1)<sup></sup> *E* exp{<sup>*tW*</sup> ˜ $j } → exp$  (5) 

*t*

 2 − *e*   

− *m*

when $(M − m + 1)2 → λ > 0 ( t$ is a real variable).

The corresponding probabilities can be expressed in terms of the confluent hypergeometric function $Φ = F$ . If *U* denotes a random variable with the compound Poisson asymptotic distribution, then for $u ≥ 1 1 1$ with $η = λ/2$.

*u l* −2η

$$
e−η  u −1η ηe P ( u)=
$$

$$
∑
$$

$$
 = Φ( U = u +1,2,η).
$$

*u*

2<sup>*u*</sup>  *l* −1 *l* \! 2

$$
l =1
$$

For example, *P* ( <sup>*U*</sup> = 0)<sup>=</sup> *e*<sup>−η</sup>, η *P* (<sup>*U*</sup> = 1)<sup>=</sup> *e*<sup>−η</sup>, 2 ηe<sup>−η</sup> $P ( U = 2)= &#91;η + 2&#93;, 8$ 

2

$$
ηe−η η  P ( U = 3)=  + η + 1, 8  6  
$$

3

ηe<sup>−η</sup> η η<sup>2</sup> 3η  *P* (<sup>*U*</sup> = 4)<sup>=</sup>  + + + 1. 16 24 2 2  More accurate, but also more complicated, expressions are given in [3]. These expressions yield the following probabilities:

| π ~0~ | 0.364091 |
| --- | --- |
| π 1 | 0.185659 |
| π2 | 0.139381 |
| π 3 | 0.100571 |
| π4 | 0.0704323 |
| π 5 | 0.139865 |
