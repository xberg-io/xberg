354 Chapter 8 | Confidence Intervals

cases, it is common to use the sample standard deviation, *s* , as an estimate of *σ* . The normal distribution creates accurate confidence intervals when *σ* is known, but it is not as accurate when *s* is used as an estimate. In this case, the Student's t-distribution is much better. Define a t-score using the following formula: - *x* - *µ t* = *s n* The *t* -score follows the Student's t-distribution with *n* – 1 degrees of freedom. The confidence interval under this

- ⎛ ⎞ *s α* distribution is calculated with *x* ± *t*~*α*~ where *t*~*α*~ is the *t* -score with area to the right equal to , *s* is the sample ⎝ ⎠ *n* 2

2 2 standard deviation, and *n* is the sample size. Use a table, calculator, or computer to find *t*~*α*~ for a given *α* . 2

**8.3 A Confidence Interval for A Population Proportion**

Some statistical measures, like many survey questions, measure qualitative rather than quantitative data. In this case, the population parameter being estimated is a proportion. It is possible to create a confidence interval for the true population proportion following procedures similar to those used in creating confidence intervals for population means. The formulas are slightly different, but they follow the same reasoning.

Let *p′* represent the sample proportion, *x/n* , where *x* represents the number of successes and *n* represents the sample size. Let *q′* = 1 – *p′* . Then the confidence interval for a population proportion is given by the following formula: p'q' p'q' p' - *Z*~*α*~ ≤ *p* ≤ p' + *Z*~*α*~ *n n*

**8.4Calculating the Sample Size n: Continuous and Binary Random Variables**

Sometimes researchers know in advance that they want to estimate a population mean within a specific margin of error for a given level of confidence. In that case, solve the relevant confidence interval formula for *n* to discover the size of the sample that is needed to achieve this goal:

2

*Z*~*α*~*σ*<sup>2</sup> *n* =

⎛- ⎞2

*x* - *µ*

⎝ ⎠

with a specific tolerance level is given by 2 *Z*~*α*~pq *n* = *e*<sup>2</sup>

# FORMULA REVIEW

**8.2 A Confidence Interval for a Population Standard Deviation Unknown, Small Sample Case**

*s* = the standard deviation of sample values. - *x* − *µ t* = is the formula for the *t* -score which measures *s n* how far away a measure is from the population mean in the Student's t-distribution *df* = *n* - 1; the degrees of freedom for a Student's tdistribution where n represents the size of the sample *T* \~ *t*~*df*~ the random variable, *T* , has a Student's t-distribution with *df* degrees of freedom

This OpenStax book is available for free at http://cnx.org/content/col11776/1.33

If the random variable is binary then the formula for the appropriate sample size to maintain a particular level of confidence

The general form for a confidence interval for a single mean, population standard deviation unknown, and sample size less than 30 Student's t is given by:

- ⎛ ⎞ - ⎛ ⎞

*s s x* - *t*~v,α~ ≤ *µ* ≤ *x* + *t* ⎝ *n*<sup>⎠</sup> ~v,α~⎝ *n*<sup>⎠</sup>

**8.3 A Confidence Interval for A Population Proportion**

*x p′=* where *x* represents the number of successes in a *n* sample and *n* represents the sample size. The variable *p* ′ is the sample proportion and serves as the point estimate for the true population proportion. *q* ′ = 1 – *p* ′
