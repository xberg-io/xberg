|  | ∞  | 1  |
| --- | --- | --- |
| p | ≈ ∏  1 − |  = 0.2888... , |
| M |  | 2 j  |
|  | j = 1 |  |
| p M − 1 ≈ 2 p M ≈ 0.5776... , |  |  |
|  | 4 pM |  |
| pM | ≈ | ≈ 0.1284... |
|  | − 2 | 9 |

and all other probabilities are very small (≤ 0.005) when M ≥ 10.

For the *N* square matrices obtained, their ranks *R*~*l*~, *l* = 1,…, *N* are evaluated, and the frequencies *F* , *F* , and *N* − *F* − *F* of the values *M* , *M* -1 and of ranks not exceeding *M* -2 are determined: *M M* −1 *M M* −1 *F* =\#{ *R*~*l*~ = *M* }<sup>,</sup>

*M*

*F*~*M*~ −1 =\#{ *R*~*l*~ = *M* −1}<sup>.</sup>

2

To apply the Χ -test, use the classical statistic

2 2 2

# (<sup>*F*</sup>

− 0.2888 *N*) (<sup>*F*</sup> − 0.5776 *N*) (<sup>*N*</sup> − *F* − *F* − 0.1336 *N*)

2 *M M* −1 *M M* −1

Χ = + + ,

0.2888 *N* 0.5776 *N* 0.13336 *N*

2

which, under the null (randomness) hypothesis, has an approximate Χ -distribution with 2 degrees of

freedom. The reported *P-value* is exp ( *obs* )/2

{<sup>−Χ2</sup> }<sup>.</sup>

Interpretation of this test: large values of Χ<sup>2</sup>( *obs* ) indicate that the deviation of the rank distribution from that corresponding to a random sequence is significant. For example, pseudo random matrices produced by a shift-register generator formed by less than *M* successive vectors systematically have rank *R*~*l*~ ≡ *M* , while for truly random data, the proportion of such occurrences should be only about 0.29.

**References for Test**

- [1] George Marsaglia, DIEHARD: a battery of tests of randomness. http://www.stat.fsu.edu/pub/diehard/.
- [2] I. N. Kovalenko (1972), "Distribution of the linear rank of a random matrix," Theory of Probability and its Applications. 17, pp. 342-346.
- [3] G. Marsaglia and L. H. Tsay (1985), "Matrices and the structure of random number sequences," Linear Algebra and its Applications. Vol. 67, pp. 147-156.

**3.6 Discrete Fourier Transform (Specral) Test**

The test described here is based on the discrete Fourier transform. It is a member of a class of procedures known as spectral methods. The Fourier test detects periodic features in the bit series that would indicate a deviation from the assumption of randomness.

*th*

Let *x*~*k*~ be the *k* bit, where *k* = 1, ..., *n* . Assume that the bits are coded -1 and +1. Let
