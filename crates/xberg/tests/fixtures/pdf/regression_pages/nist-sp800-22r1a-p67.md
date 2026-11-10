K=6, M=10000

2

| classes | probabilities |
| --- | --- |
| {ν≤ 10} | π 0 = 0.0882 |
| {ν=11} | = 0.2092 π1 |
| {ν=12} | π 2 = 0.2483 |
| {ν=13} | π ~3~ = 0.1933 |
| {ν=14} | π 4 = 0.1208 |
| {ν=15} | = 0.0675 π5 |
| {ν≥ 16} | = 0.0727 π6 |

Large values of Χ indicate that the sequence has clusters of ones; the generation of "random'" sequences by humans tends to lead to small values of ν~*n*~ (see [3, p. 55]).

**References for Test**

- [1] F. N. David and D. E. Barton, Combinatorial Chance. New York: Hafner Publishing Co., 1962, p. 230.
- [2] Anant P. Godbole and Stavros G. Papastavridis (ed), Runs and Patterns in Probability: Selected Papers. Dordrecht: Kluwer Academic, 1994.
- [3] Pal Revesz, Random Walk in Random and Non-Random Environments. Singapore: World Scientific, 1990.

**3.5 Binary Matrix Rank Test**

Another approach to testing for randomness is to check for linear dependence among fixed-length substrings of the original sequence: construct matrices of successive zeroes and ones from the sequence, and check for linear dependence among the rows or columns of the constructed matrices. The deviation of the rank - or rank deficiency - of the matrices from a theoretically expected value gives the statistic of interest.

This test is a specification of one of the tests coming from the DIEHARD [1] battery of tests. It is based on the result of Kovalenko [2] and also formulated by Marsaglia and Tsay [3]. The result states that the rank *R* of the *M x Q* random binary matrix takes values *r* = 0, 1, 2, …, *m* where *m* ≡ *min* ( *M* , *Q* ) with probabilities

*r* −1(<sup>1</sup>− 2i−Q)(<sup>1</sup>− 2i−M) *r* ( *Q* + *M* − *r* )− *MQ*

*p*~*r*~ = 2 .

∏

*i* − *r*

# 1− 2

*i* = 0

The probability values are fixed in the test suite code for *M* = *Q* = 32. The number *M* is then a parameter

of this test, so that ideally *n* = *M*<sup>*2*</sup>*N* , where *N* is the new "sample size." In practice, values for *M* and *N* are

chosen so that the discarded part of the string, *n* - *NM*<sup>*2*</sup> , is fairly small.

The rationale for this choice is that
