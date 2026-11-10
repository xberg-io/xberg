**the matrix inequality in (6) can be written as**

 

**T**

**Y B**~**d**~ **Y B**~**u**~ **C**~**z**~

**T P + P**

 0 0  0  

**−2**

 

**W**

0 0

**≤ 0,**



**d**



**∗ −γ**

  0

**diag (κ )**

0

**a**

**∗ ∗ −γI which is convex in Y , V , ω**~**c**~**, and κ**~**a**~**, where Y A Y B**~**u**~ **P := . V −diag(ω**~**c**~**)**

**where λ**~**a**~ **≥ 0, λ**~**ω**~ **≥ 0, and λ**~**x**~ **≥ 0 are user-defined c F weights, and W**~**d**~ **is a given scaling matrix for d(t). The**

**−1**

**controller gain can be recovered as K :=** diag **(ω**~**c**~**) V . Proof. The condition for bounding the H**~**2**~ **norm**

¯

**d from → z is given by XM**~**11**~ **+ w**~**a**~

¯

**2 W 0 T d T**

**M X + XM**~**12**~ **M X ≤ 0, and 11 2 12**

**0 W**~**a**~

h i

**−1 T**

**tr M**~**21**~**X M**~**21**~ **≤ γ [15], which for the problem considered are the following LMIs**

 

**The DC gain matrix of the actuator is minimized, by T Y B**~**d**~ **Y B**~**u**~

h i

**P + P T**

 0 0 

**minimizing a upper bound of tr V V , i.e., minimize γ**~**xF**~

 

**≤ 0, −2**

 

**T W**

0

**d**

**such that V V ≤ Q and tr [Q] ≤ γ**~**x**~ **and Q ≥ 0. The F ∗ −**

0

**diag (κ**~**a**~**) constraint on V can be expressed as the following LMI using**

 

**Schur complement, Q**~**1**~ **C**~**z**~

0  

**T**

**∗ Y 0 ≥ 0, tr [Q**~**1**~**] ≤ γ.**

**Q V ≥ 0.**

**∗ ∗ I**

### V I

h i

**T The actuator noise is maximized by minimizing kκ k , the Similar to theorem 1, tr V V ≤ γ is equivalent**

**a 2 x**~**F**~

**control rate is minimized by minimizing kω k , and the con- T c 2**

**Q**~**2**~ **V to ≥ 0 and tr [Q ] ≤ γ . Maximum actrol magnitude is minimized by minimizing γ . Therefore,**

**xF x**~**F**~

**the cost function for the optimization is min λ**~**a**~**kκ k**~**a**~ **+ λ**~**ω**~ **kω k**~**c**~ **+ λ**~**x**~ **γ , 2 c 2 F x**~**F**~

**κ**~**a**~**,ω**~**c**~**,γ**~**x**~ **,V ,Y ≥0,Q≥0 F**

**where λ**~**a**~**, λ**~**ω**~ **, and λ**~**x**~ **are user-defined weights. c F**

**2**

### V I

**tuator degradation is achieved by minimizing λ**~**a**~**kκ k**~**a**~ **+** ~**2**~ **λ**~**ωc**~ **kω k**~**c**~ **+ λ**~**xF**~**γ**~**xF**~**, where λ**~**a**~**, λ**~**ωc**~**, and λ**~**xF**~ **are user2 defined weights.**

**Remark 3. In both theorem 1 and 2, the minimum control rate is given by the elements ω**~**c**~**, the minimum control magnitude**

**We next present the result for H**~**2**~ **optimal closed-loop is characterized by γ**~**x**~ **, and the maximum actuator noise is F √ performance. characaterized by the scaling** diag **1/ κ , where the square-**

**Theorem 2. For the system in (3) with D = 0, the state- root and the inverse are defined to be element-wise.**

**d**

**d Therefore, the optimal ω**~**c**~**, γ**~**x**~ **, and κ quantities maximum F feedback gain that guarantees → z ≤ γ for a w**~**a**~ **actuator degradation (as defined above) for which a state-**

**2 given γ ≥ 0, with maximum actuator noise, and minimum feedback controller is able to achieve the user specifiedcontrol magnitude and rate, is given by the following convex optimization problem,**

**min λ**~**a**~**kκ k**~**a**~ **+ λ**~**ω**~ **kω k**~**c**~ **+ λ**~**x**~ **γ , 2 c p F x**~**F**~

**κ ,ω ,γ ,V ,Y ,Q**~1~**,Q a c x** ~2~ **F**

**subject to**

 

**Y B**~**d**~ **Y B**~**u**~

**T P + P**

 0 0   

**≤ 0 −2**

 

**W**

0

**d ∗ −**

0 diag **(κ**~**a**~**)**  

**Q**~**1**~ **C**~**z**~

0

**T**

**Q**~**2**~ **V**

 

**∗ Y 0 ≥ 0, ≥ 0,**

### V I

**∗ ∗ I nu nu**

tr **[Q ] ≤ γ,** tr **[Q ] ≤ γ , κ ∈** R **, ω ∈** R **, γ ∈** R~**+**~**,**

**1 2 x**~**F**~ **+ c + x**~**F**~ **nu×nx nx nz nx**

**V ∈** R **, Y ∈** S **, Q ∈** S~**+**~ **, Q**~**2**~ **∈**

S~**+**~

**, + 1**

**with Y A Y B**~**u**~ **P := , V −** diag **(ω**~**c**~**)**

**performance in terms of the closed-loop systems H**~**2**~ **or H**~**∞**~ **norms.**

- IV. **EXAMPLES**

- A. **F-16 Flight Control Application We apply the results to the design of a full-state feedback controller for a model of the longitudinal motion of the F-16 aircraft. The model is based on the aerodynamics from the NASA Technical Report 1538 [16]. The nonlinear model is trimmed at 10000 ft at 900 ft/s for steady-level flight. The longitudinal states are θ (pitch angle), V**~**t**~ **(total velocity), α**

- 
**¯ (angle-of-attack), q (pitch rate); with trim value θ = 5.95 ,**

- 
**¯V**~**t**~ **= 900 ft/s, α¯ = 5.95 , q¯ = 7.85 deg/s. The control variables are T (thrust), δ**~**e**~ **(elevator angle), and δ**~**lef**~ **(leading**

- 
**¯ ¯ flap); with trim values T = 10461.84 lb, δ**~**e**~ **= −3.82 , and**

- 
**δ**~**lef**~ **= 12.42 . We assume the aircraft experiences disturbance in the angle-of-attack α, with W**~**d**~ **= 0.01. The open-loop system in (2) is given by the following system matrices**
