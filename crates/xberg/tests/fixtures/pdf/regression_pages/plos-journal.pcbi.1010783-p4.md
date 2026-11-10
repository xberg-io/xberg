PLOS COMPUTATIONAL BIOLOGY

Adjoint sensitivities at steady-state in ODE models of biochemical reaction networks

a steady state. This comes at a high computational cost and scales poorly with the number of states and parameters, as one has to simulate a system of ODEs with the dimension (n~x~+ 1)n~θ~. The existing adjoint method for pre- and post-equilibration simulates the model until approximate convergence to a steady state, and then solves the corresponding backward equation for the same time interval [12, 27]. This is computationally costly as well but possesses, in contrast to the forward method, improved scalability; here, one has to sequentially simulate two systems of the size n~x~. However, the forward method can be easily adapted to exploit steady-state constraints. One can find state sensitivities at steady state without numerical integration, by solving a system of linear equations per parameter (see, e.g., [20]). The downside is that this approach is not applicable for systems with singular Jacobians. A corresponding approach for adjoint methods is currently missing. In this paper, we introduce a novel adjoint method for the computation of objective function gradients for problems with steady-state constraints. The proposed method exploits the steady-state constraint to circumvent numerical backward integration for pre- and post-equilibration. Instead, a system of equations is solved to compute parts of the objective function gradient. We provide an implementation of the method in the open-source toolbox AMICI and investigate its accuracy and efficiency on three real-world problems. Our study demonstrates that the new method achieves a substantial speedup for gradient computation, which is particularly valuable for large models.

**Methods**

In this section, we introduce the considered classes of mathematical models and parameter estimation problems. Subsequently, we describe the established method for gradient calculation via adjoint sensitivity analysis (ASA) and propose a new formulation for the steady-state case. We discuss how the method can be applied to models of biochemical reaction networks. The method, however, is not restricted to the field of computational biology. It can be used for any model of the form described in this section with pre- or post-equilibration, or both.

**Mathematical modeling and simulation**

We consider ODE models of the form

**x**

_ ¼ **fðxðtÞ** *;* **θ** *;* **uÞ** *;* **xðt**~0~Þ¼ **x**~0~ðθ *;* **uÞ** *;* ð1Þ in which **xðt** *;* **θ** *;* **uÞ** 2 Rnþx is the vector of state variables, **θ** 2 Rnþy is the parameter vector, **u** 2 Rnþu is the vector of constant inputs and t 2 R is the time. The vector field of the ODE model is **f** : Rnþx Rnþy Rnþu \! Rnþx, which is Lipschitz-continuous with respect to x, and the initial condition at t~0~is **x**~0~ : Rnþy Rnþu \! Rnþx. The ODE model determines the time evolution of the state variables, given the parameters. In case of biochemical reaction networks, this can be e.g., concentrations of biochemical species and reaction rate constants, respectively. The inputs ( **u** ) describe experimental conditions, e.g., regime of administration of a drug or composition of a cell culture medium. In many models, the state variables approach a steady-state **x** ( **θ** , **u** ) for large values of t: **x** ðθ *;* **uÞ** ¼ lim **xðt** *;* **x**~0~ðθ *;* **uÞ** *;* **θ** *;* **uÞ** *:* ð2Þ

t\!1

In this paper we assume that for each initial condition ( **x**~0~( **θ** , **u** )) there exists an exponentially stable steady state. Both pre- and post-equilibration cases assume existence of a steady state that the dynamical system can reach fast enough to appropriately describe the modeled

PLOS Computational Biology | https://doi.org/10.1371/journal.pcbi.1010783 January 3, 2023 4 / 19
