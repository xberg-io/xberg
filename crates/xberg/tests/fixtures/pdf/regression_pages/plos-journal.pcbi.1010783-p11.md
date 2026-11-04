PLOS COMPUTATIONAL BIOLOGY

Adjoint sensitivities at steady-state in ODE models of biochemical reaction networks

**Results**

In this section we investigate whether using the proposed ssASA method is beneficial, compared to the standard ASA approach, to computing the objective function gradient. In both cases we used ASA, but in the first case gradient computation was done via numerical integration of the ODE system (9), whereas in the second case the new approach was applied, where the system of Eqs (15) and (17) is solved. In both cases numerical integration was used to find the steady state of the system (1), i.e. to find the solution

**x** ( **θ** , **u** ) of **x**

_ ¼ **fðx** ðθ *;* **uÞ** *;* **θ** *;* **uÞ** ¼ **0** . The two cases will be referred to as "standard ASA for sensitivities" and "ssASA for sensitivities". We assess accuracy by comparing the gradient values computed with the two methods, as well as the difference in computation time. The comparison is done on three previously published ODE-constrained optimization problems of varying complexity that required either pre- or post-equilibration (Table 1). The problems were taken from the PEtab Benchmark Collection, a GitHub repository with parameter estimation problems from published studies implemented in the PEtab format [11, 42]. The Blasi et al., 2016 model describes acetylation of the histone H4 [13]. It is linear and has 16 state variables and 9 optimized parameters, which is the dimension of objective function gradient (8). Only steady-state data is available for this model. The Blasi et al., 2016 model contains conserved quantities, which were automatically removed during model import by AMICI to ensure non-singularity of the Jacobian. The model by Zheng et al., 2012 is also linear and describes histone H3 methylation pattern formation. For this model dynamic data is available and the initial condition is determined by pre-equilibration. The model by Fro¨hlich et al., 2018 is a large-scale nonlinear model describing the drug response of cancer cell lines [12]. The measurement data used for optimization was collected when the system was assumed to be at steady state.

**Accuracy with ssASA is preserved**

To investigate the accuracy of objective function gradients computed with the proposed ssASA method, we performed 500 model simulations with different parameter vectors sampled uniformly within the parameter bounds specified in the parameters PEtab file [11]. For each model, the same initial state, specified in the PEtab files, was used for each simulation and no simulation failed. For all three models the computed objective function gradients were very similar for both compared approaches (Fig 3). The maximum and median deviations (as defined in the S1

−4

Text, Section "Accuracy of gradient computation") are respectively equal to 1.9 10 and

−11 −17

numerically 0 for the Blasi et al., 2016 model, 2.8 10 and 1.6 10 for the Zheng et al.,

−2 −20

2012 model, 3.1 10 and 3.04 10 for the Fro¨hlich et al., 2018 model. Therefore, we conclude that the new approach gives accurate results.

| Table 1. Overview of the models and optimization problems considered in this study. Problem | n θ | nx | \# data points | Equilibration type | Reference |
| --- | --- | --- | --- | --- | --- |
| Blasi et al ., 2016 | 9 | 16 | 288 | post-equilibration | [13] |
| Zheng et al ., 2012 | 46 | 15 | 60 | pre-equilibration | [14] |
| Fro ¨hlich et al ., 2018 | 4088 | 1396 | 143 | post-equilibration | [12] |
| https://doi.org/10.1371/journal.pcbi.1010783.t001 |  |  |  |  |  |

**x**

PLOS Computational Biology | https://doi.org/10.1371/journal.pcbi.1010783 January 3, 2023 11 / 19
