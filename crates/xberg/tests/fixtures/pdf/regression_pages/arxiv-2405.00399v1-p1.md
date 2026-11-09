# Enhanced Error Estimates for Augmented Subspace Method

with Crouzeix-Raviart Element

∗ † §

### Zhijin Guan, Yifan Wang, Hehu Xie<sup>‡</sup> and Chenguang Zhou

**Abstract**

In this paper, we present some enhanced error estimates for augmented subspace methods with the nonconforming Crouzeix-Raviart (CR) element. Before the novel estimates, we derive the explicit error estimates for the case of single eigenpair and multiple eigenpairs based on our defined spectral projection operators, respectively. Then we first strictly prove that the CR element based augmented subspace method exhibits the second-order convergence rate between the two steps of the augmented subspace iteration, which coincides with the practical experimental results. The algebraic error estimates of second order for the augmented subspace method explicitly elucidate the dependence of the convergence rate of the algebraic error on the coarse space, which provides new insights into the performance of the augmented subspace method. Numerical experiments are finally supplied to verify these new estimate results and the efficiency of our algorithms.

**Keywords.** eigenvalue problem, augmented subspace method, enhanced error estimate, Crouzeix-Raviart element.

**AMS subject classifications.** 65N30, 65N25, 65L15, 65B99.

## 1 Introduction

Solving large-scale eigenvalue problems is a basic and challenging task in the field of scientific and engineering computing. Compared with linear boundary value problems, it is always more difficult to solve eigenvalue problems because it requires more computational work and memory. In order to solve these large-scale sparse eigenvalue problems, Krylov subspace type methods (Implicitly Restarted Lanczos/Arnoldi Method (IRLM/IRAM) [25]), Preconditioned INVerse ITeration (PINVIT) method [7, 15, 18], Locally Optimal Block Preconditioned Conjugate Gradient

- (LOBPCG) method [19, 20], Jacobi-Davidson-type techniques [3] and Generalized Conjugate Gradient Eigensolver (GCGE) [21, 32] have been developed. All these popular methods include the orthogonalization steps during computing Rayleigh-Ritz problems which are always the bottlenecks for designing efficient parallel schemes for determining relatively many eigenpairs.

∗LSEC, ICMSEC, Academy of Mathematics and Systems Science, Chinese Academy of Sciences, Beijing 100190, China, and School of Mathematical Sciences, University of Chinese Academy of Sciences, Beijing 100049, China. ( guanzhijin@lsec.cc.ac.cn ) †

LSEC, ICMSEC, Academy of Mathematics and Systems Science, Chinese Academy of Sciences, Beijing 100190, China, and School of Mathematical Sciences, University of Chinese Academy of Sciences, Beijing 100049, China. ( wangyifan@lsec.cc.ac.cn ) ‡LSEC, ICMSEC, Academy of Mathematics and Systems Science, Chinese Academy of Sciences, Beijing 100190, China, and School of Mathematical Sciences, University of Chinese Academy of Sciences, Beijing 100049, China. ( hhxie@lsec.cc.ac.cn ) §Department of Mathematics, Beijing University of Technology, Beijing 100124, China. (Corresponding author: zhoucg@bjut.edu.cn )

1
