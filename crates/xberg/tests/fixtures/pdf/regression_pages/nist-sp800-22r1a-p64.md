1/2. The statistic is referred to a chi-squared distribution with the degrees of freedom equal to the number of substrings.

The parameters of this test are *M* and *N* , so that *n* = *MN* , i.e., the original string is partitioned into *N* substrings, each of length *M* . For each of these substrings, the probability of ones is estimated by the observed relative frequency of 1's, π~*i*~, *i* = 1,…, *N* . The sum

*value* is

2 *N*

 1

Χ<sup>2</sup>( *obs* ) = 4 *M*∑<sup>π</sup> − ~~

*i*

  2

*i* =1

under the randomness hypothesis has the Χ<sup>2</sup>-distribution with N degrees of freedom. The reported *P-*

| ∫ ∞ | 2 − 1 | ∫ ∞ | − 1 |  |  |
| --- | --- | --- | --- | --- | --- |
| e − u / uN / 2 | du | e − uu N / 2 | du9 | N Χ ( obs ) |  |
| Χ 2 ( obs ) |  | 2 ( obs )/ 2 |  | 2 |  |
|  | = | Χ | = igamc ( | , | ) . |
| Γ ( N /2)2 N / 2 |  | Γ ( N /2) |  | 2 2 |  |

**References for Test**

- [1] Nick Maclaren, "Cryptographic Pseudo-random Numbers in Simulation," Cambridge Security Workshop on Fast Software Encryption. Dec. 1993. Cambridge, U.K.: R. Anderson, pp. 185-190.
- [2] Donald E. Knuth, The Art of Computer Programming. Vol 2: Seminumerical Algorithms. 3rd ed. Reading, Mass: Addison-Wesley, 1998 (especially pp. 42-47).
- [3] Milton Abramowitz and Irene Stegun, Handbook of Mathematical Functions: NBS Applied Mathematics Series 55. Washington, D.C.: U.S. Government Printing Office, 1967.

**3.3 Runs Test**

This variant of a classic nonparametric test looks at "runs" defined as substrings of consecutive 1's and consecutive 0's, and considers whether the oscillation among such homogeneous substrings is too fast or too slow.

The specific test used here is based on the distribution of the total number of runs, *V*~*n*~. For the fixed proportion π = ∑ *n* (which by the Frequency test of Section 3.1 must have been established to be ε~*j*~ /

*j*

1 2 close to 0.5: π − ≤ ). 2 *n* lim *P* ( *V*~*n*~ − 2 *nπ* (1− π)

*n* →∞

2 *nπ* (1− π) To evaluate *V*~*n*~, define for k = 1,…,n-1, *r* ( *k* ) = 0 if ε~*k*~ = ε and *r* ( *k* ) = 1 if ε ≠ ε . Then

*n* −1

*V* = *k* ) + 1. The *P-value* reported is

*n*

∑

*r* (

*k* =1

*V* ( *obs* ) − 2 *nπ* (1− π)

*n*

*erfc* ( 2 2 *nπ* (1− π)

≤ *z* ) = Φ( *z* ).

*k* +1 *k*

).

(2)

*k* +1
