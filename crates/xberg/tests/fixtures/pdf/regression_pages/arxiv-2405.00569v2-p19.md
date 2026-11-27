Central compact schemes for third derivatives 19

```
0
-1.91
0
-1.91
-0.2 -1.92 -0.2 -1.92
-0.4 -0.4
-1.93 -1.93
-0.6 -0.6
-1.94 -1.94
-0.8 -0.8
-1.95 -1.95
-1 -1
```

```
u(x,t)
u(x,t)
u(x,t)
```

**-1.96 u(x,t)**

```
-1.96
-1.2 -1.2
-1.4 -1.97 -1.4 -1.97
-1.6
Exact-CNCS -1.98
Exact-CNCS
-1.8
F-0
F-8
F-0
F-8
F-10 F-10 F-10 F-10
-1.99 -1.99
-2 F-12 F-12 -2 F-12 F-12
-10 -5 0 5 10 0.35 0.4 0.45 0.5 0.55 0.6
x x
```

| -1.6 Exact-CNCS | Exact-CNCS -1.98 |
| --- | --- |
| F-0 | F-0 |
| -1.8 | F-8 |

| -2 |  |  |  | -2 |
| --- | --- | --- | --- | --- |
| 15 0.3 | 0.35 0.4 0.45 0.5 0.55 | -10 0.6 | -5 0 5 10 | 15 0.3 |
|  | x |  | x |  |

- (a) TDCNCS, filtered ev- (b) Zoom in near smooth (c) TDCNCS, filtered ev- (d) Zoom in near smooth

ery 2 time steps extrema ery 10 time steps extrema

```
0
-1.91
0
-1.91
-0.2 -1.92 -0.2 -1.92
-0.4 -0.4
-1.93 -1.93
-0.6 -0.6
-1.94 -1.94
-0.8 -0.8
-1.95 -1.95
-1 -1
```

```
u(x,t)
u(x,t)
u(x,t)
```

**-1.96 u(x,t)**

```
-1.96
-1.2 -1.2
-1.4 -1.97 -1.4 -1.97
-1.6
Exact-CCS -1.98
Exact-CCS
-1.8
F-0
F-8
F-0
F-8
F-10 F-10 F-10 F-10
-1.99 -1.99
-2 F-12 F-12 -2 F-12 F-12
-10 -5 0 5 10 0.35 0.4 0.45 0.5 0.55 0.6
x x
```

| -1.6 Exact-CCS | Exact-CCS -1.98 |
| --- | --- |
| F-0 | F-0 |
| -1.8 | F-8 |

| -2 |  |  |  | -2 |
| --- | --- | --- | --- | --- |
| 15 0.3 | 0.35 0.4 0.45 0.5 0.55 | -10 0.6 | -5 0 5 10 | 15 0.3 |
|  | x |  | x |  |

- (e) TDCCS, filtered every (f) Zoom in near smooth (g) TDCCS, filtered every (h) Zoom in near smooth 2 time steps extrema 10 time steps extrema

Figure 9. Analysis of filter at $N = 40, t = 0.1$ for Example 7.2.

where $\epsilon=10^{-4}$. The solution is simulated over the interval t ∈ [0, 4] using periodic boundary conditions, employing eighth-order TDCNCS and TDCCS schemes with a grid size of N = 150 and time step $∆t=0.5∆x^{2}$ as in [11]. Figure (12) illustrates the solution at time instances t = 0, 1, 2, and its contour plot up to t = 4. Notably, TDCNCS exhibits small-scale numerical oscillations, whereas TDCCS produces a smooth numerical solution. To address the presence of high-frequency oscillations, we apply a 12th-order periodic filter with α = 0.4 every 20 steps for TDCNCS and every 50 steps for TDCCS (time steps = 20000). Our experiments indicate that filtering every 50 steps does not sufficiently eliminate the small oscillations in TDCNCS.

**Example 7.4.** In this example, we investigate the zero dispersion limit of conservation laws, specifically the

KdV equation (7.3) with the continuous initial condition (7.7)

$$
u_{0}(x)=2+\frac{1}{2}\sin(2\pi x),x\in[0,1],
$$

with periodic boundary conditions. Theoretical and numerical analysis concerning the limit as $\epsilon\rightarrow0^{+}$ are available in [**?** ] and [64]. The primary goal is assessing our numerical method capability to resolve small-scale solution structures in this limit when ϵ is small. To address this, we compute solutions for t = 0.5 with

−7

$\epsilon=10^{-4},10^{-5},10^{-6}$, and 10 using eighth-order TDCNCS and TDCCS, with a step size $∆t=∆x^{2}$ as in [11]. Since the exact solution is not known for this problem, we utilize a reference solution acquired through a significantly higher number of grid points (N = 1000) using TDCNCS. For $\epsilon=10^{-4}$ (with N = 100) and $\epsilon=10^{-5}$ (with N = 200), Figures (13)(a) and (13)(b) depict the solutions obtained by TDCNCS (in red), TDCCS (in blue), in comparison with the reference solution (in black). Similarly, for $\epsilon=10^{-6}$ (with N = 800) and $\epsilon=10^{-7}$ (with N = 1600) using both TDCNCS and TDCCS, Figures (13)(c)-(f) showcase these solutions, confirmed as "converged", exhibiting physical oscillations typical in dispersive limits [64]. Moreover, inadequate mesh refinement results in failure to achieve a converged solution. For instance, when $\epsilon=10^{-6}$, we observed that numerical solutions obtained with 200, 300, 400, and even 600 uniform cells did
