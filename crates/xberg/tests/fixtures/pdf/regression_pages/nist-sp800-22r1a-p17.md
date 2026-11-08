**1.1.6 Considerations for Randomness, Unpredictability and Testing**

The following assumptions are made with respect to random binary sequences to be tested:

1. Uniformity: At any point in the generation of a sequence of random or pseudorandom bits, the occurrence of a zero or one is equally likely, i.e., the probability of each is exactly 1/2. The expected number of zeros (or ones) is *n* /2, where *n* = the sequence length.
2. Scalability: Any test applicable to a sequence can also be applied to subsequences extracted at random. If a sequence is random, then any such extracted subsequence should also be random. Hence, any extracted subsequence should pass any test for randomness.
3. Consistency: The behavior of a generator must be consistent across starting values (seeds). It is inadequate to test a PRNG based on the output from a single seed, or an RNG on the basis of an output produced from a single physical output.

**1.2 Definitions**

| Term | Definition |
| --- | --- |
| Asymptotic Analysis | A statistical technique that derives limiting approximations for functions of interest. |
| Asymptotic Distribution | The limiting distribution of a test statistic arising when n approaches infinity. |
| Bernoulli Random | A random variable that takes on the value of one with probability p and the |
| Variable | value of zero with probability 1-p. |
| Binary Sequence | A sequence of zeroes and ones. |
| Binomial Distribution | A random variable is binomially distributed if there is an integer n and a probability p such that the random variable is the number of successes in n independent Bernoulli experiments, where the probability of success in a single experiment is p. In a Bernoulli experiment, there are only two possible outcomes. |
| Bit String | A sequence of bits. |
| Block | A subset of a bit string. A block has a predetermined length. |
| Central Limit Theorem | For a random sample of size n from a population with mean µ and variance σ2, the distribution of the sample means is approximately normal with mean µ and variance σ2/n as the sample size increases. |
| Complementary Error Function | See Erfc. |
| Confluent Hypergeometric | The confluent hypergeometric function is defined as |
| Function | 1 Γ(b) −1 Φ(a;b;z) = t)b−a dt,0 \< a \< b. ∫ e zt t a−1(1− Γ(a)Γ(b − a) 0 |
| Critical Value | The value that is exceeded by the test statistic with a small probability (significance level). A "look-up" or calculated value of a test statistic (i.e., a test statistic value) that, by construction, has a small probability of |
| € | occurring (e.g., 5 %) when the null hypothesis of randomness is true. |
| Cumulative Distribution | A function giving the probability that the random variable X is less than or |
| Function (CDF) F(x) | equal to x, for every value x. That is, F(x) = P(X ≤ x). |
