PLOS COMPUTATIONAL BIOLOGY

Estimating parameter distributions with repeated cross-sectional data

12 months, thereby indicating that when the mouse reaches a certain rate of amyloid Aβ40 or 42 early in their life, it cannot survive for a long time.

**Target cell-limited model**

We fitted the target cell-limited model to the virus dataset, obtained from [25]. In the dataset, daily viral loads V were measured from groups of BALB/cJ mice infected with influenza A/ Puerto Rico/8/34 (H1N1) virus (PR8) (Fig 9, left). The mice received an intranasal administration of a dose of 75 TCID50 of the PR8 virus at the initial time point (t = 0), where TCID50 is the concentration required to infect 50% of the cell cultures [32, 33]. Unlike the previous estimation tasks, only the value of V is observable out of all the populations in the model, thus the parameter estimation was performed using only the viral loads. Data was collected over 12 days, with 10 animals sampled per time point. For faster computation, we utilized four time points observed at 1, 3, 7, and 8 days. With this RCS data, we applied EPD for estimating the parameter distributions with a target cell-limited model with delayed virus production (Fig 9, right). Surprisingly, this result showed that two distributions of the parameters p and K~*δ*~have at least three centers. The estimation result of the remaining parameters, *β* , *κ* , c, and *δ* , were provided in (S3 Fig). Therefore, EPD not only can accurately predict parameters, but also demonstrate the heterogeneity of parameters. Thus, when it is near the value from the previous research, our results suggest the existence of multiple parameter sets that can represent this dataset, beyond those previously identified parameters.

**Discussion**

This study proposed the EPD method for inferring parameter distributions from Repeated Cross-Sectional data in systems modeling. Unlike previous approaches, which often overlooked data heterogeneity and resulted in information loss, EPD determines more precise and accurate parameter distributions across a variety of systems. By estimating parameter distributions, EPD facilitates a deeper understanding of the underlying dynamics of these systems.

**Fig 9. Parameter estimation results of a target cell-limited model with delayed production with a real experimental virus infection dataset.**

| Parameter estimation results of | a target cell-limited model with delayed |
| --- | --- |
| virus infection dataset. | Black dots represent the virus population |
| the four components in | the model (left). Artificial trajectories using |
| graph (left, red curves). Two | parameters corresponding to the artificial |

V over time t, the only measurable factor among the four components in the model (left). Artificial trajectories using EPD were simultaneously provided on the same graph (left, red curves). Two parameters corresponding to the artificial trajectories, p and K~*δ*~, were estimated (right, red dots). The other parameters, *β* , c, *κ* , and *δ* , are detailed in S3 Fig. https://doi.org/10.1371/journal.pcbi.1012696.g009

PLOS Computational Biology | https://doi.org/10.1371/journal.pcbi.1012696 December 23, 2024 12 / 22
