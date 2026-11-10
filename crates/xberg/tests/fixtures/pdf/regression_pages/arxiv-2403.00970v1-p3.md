**a continuously differentiable function** ***N*** **(** ***ζ*** **) : [0** ***,*** **∞) → This approach significantly reduces the complexity of (−∞** ***,*** **∞), it is defined via its positive and negative the gain selection process, focusing on only two key**

**+ −**

**truncated forms,** ***N*** **(** ***ζ*** **) = max{0** ***,N*** **(** ***ζ*** **)} and** ***N*** **(** ***ζ*** **) = parameters. The time-varying gain** ***κ***~**∆**~**(** ***t*** **) is adaptively max{0** ***,*** **−** ***N*** **(** ***ζ*** **)}, respectively. The function satisfies the updated by condition that**

**+ −**

***N*** **(** ***ζ*** **) =** ***N*** **(** ***ζ*** **) −** ***N*** **(** ***ζ*** **)** ***,*** **with the properties that for any** ***ζ*** **, ïZ** ***v*** **Z** ***v*** **ò 1**

**+ −**

**lim sup** ***N*** **(** ***ζ*** **)** ***dζ*** **−** ***N*** **(** ***ζ*** **)** ***dζ*** **=**

***v*** **→∞**

***v***

**0 0**

**ïZ** ***v*** **Z** ***v*** **ò 1**

**− +**

**lim sup** ***N*** **(** ***ζ*** **)** ***dζ*** **−** ***N*** **(** ***ζ*** **)** ***dζ*** **=**

***v*** **→∞**

***v***

**0 0**

***T***

***κ*** **(** ***t*** **) = −** ***αψ*** **ˆ(** ***t*** **)** ***φ*** **(** ***x*** **)** ***,*** **(10) ∆ (4) and the adaptive law for** ***ψ*** **ˆ(** ***t*** **) is given by ˙**

***ψ*** **ˆ(** ***t*** **) = −Γ (** ***αkΨ*** **(** ***t*** **)k**<sup>**2**</sup> ***φ*** **(** ***x*** **) +** ***σψ*** **ˆ(** ***t*** **))** ***.*** **(11)**

***T***

**∞** ***,*** **(5) with** ***α*** **and** ***φ*** **are positive control constants, and Γ = Γ** ***\>*** **0.**

**In this work, we choose** ***N*** **(** ***ζ*** **) =** ***ζ***<sup>**2**</sup> **cos(** ***ζ*** **) with the ∞** ***.*** **(6) property of** ***N*** **(0) = 0 as our Nussbaum function. The updating law for** ***ζ*** **is given by A function that satisfies these conditions is utilized for**

***T***

**managing the uncertainty in control direction, offering ˙** ***ζ*** **(** ***t*** **) = Ψ (** ***t*** **) (** ***k***~**∆**~**+** ***κ***~**∆**~**(** ***t*** **))Ψ (** ***t*** **)** ***.*** **(12) a robust approach to control system design. The above control framework, enhanced by adaptive** *Lemma 2:* **[16], [17] Let** ***V*** **(** ***t*** **) and** ***ζ*** **(** ***t*** **) be smooth laws based on the low complexity Nussbaum function functions defined over the interval [0** ***,t***~***f***~**) with** ***V*** **(** ***t*** **)** ***\>*** **0 PID control, provides us with an effective approach for all** ***t*** **∈ [0** ***,t***~***f***~**). Given** ***N*** **(** ***ζ*** **(** ***t*** **)) as a Nussbaum-type to robotic manipulator control, as formalized in the function, if for any** ***t*** **∈ [0** ***,t***~***f***~**), the following condition forthcoming theorem. is met: Z** *Theorem 1:* **Consider the robot manipulator system**

***t***

**˙ described by (1) with the Properties 1-3, satisfying to**

***V*** **(** ***t*** **)** ***\< c*** **+** ***e−c1t*** **(** ***gN*** **(** ***ζ*** **(** ***τ*** **)) + 1)** ***ζ*** **(** ***τ*** **)** ***ec1τdτ,*** **(7)**

**0 0**

**Assumptions 1 and 2. If the Nussbaum function-based where** ***c***~**0**~ ***\>*** **0 and** ***c***~**1**~ ***\>*** **0 are positive constants, and** ***g*** **PID-like control law (9), with the adaptive updating laws is a control parameter within closed intervals** ***L*** **= [** ***l***~**−**~***,l***~**+**~**] (11), and (12), and and Lemmas 1, and 2 is implemented, excluding zero (0** \< ***L*** **), then** ***V*** **(** ***t*** **),** ***ζ*** **(** ***t*** **), and the integral then, with the design parameters are properly chosen, R**~***t***~ **˙ the closed-loop system remains stable, and all signals** ***gN*** **(** ***ζ*** **(** ***τ*** **))** ***ζ*** **(** ***τ*** **)** ***ec1τdτ*** **are guaranteed to be bounded on**

**0**

**[0** ***,t*** **within the system are uniformly bounded. Furthermore,** ***f*****). the joint position signal** ***q*** **(** ***t*** **) closely tracks the desired**

- III. **Nussbaum Function-based Control Design and**

**trajectory** ***q***~***d***~**(** ***t*** **), with the tracking error converging to a Stability Analysis small neighborhood around zero. We propose a Nussbaum function based PID-like con-** Proof: **Consider the Lyapunov candidate as trol input for robot manipulator control, delineated as: 1 1 Z**

***T*** **−1**

**˜ ˜**

***t***

***V*** **(** ***t*** **) = Ψ (** ***t*** **)** ***M*** **(** ***q*** **(** ***t*** **))Ψ (** ***t*** **) +** ***ψ*** **(** ***t*** **)**<sup>***T***</sup> **Γ** ***ψ*** **(** ***t*** **)** ***,*** **(13)**

**2 2** ***u*** **(** ***t*** **) =(** ***k***~***π***~**+** ***κ***~***π***~**(** ***t*** **))** ***K***~***N***~**(** ***ζ*** **)** ***e*** **(** ***t*** **) + (** ***k***~***ι***~**+** ***κ***~***ι***~**(** ***t*** **))** ***K***~***N***~**(** ***ζ*** **)** ***e*** **(** ***ρ*** **)** ***dρ***

**0 ∗ ∗**

**where** ***ψ*** **˜(** ***t*** **) =** ***ψ*** **ˆ(** ***t*** **) −** ***ψ*** **with** ***ψ*** **(** ***t*** **) is the ideal constant** ***d*** **weight vector of neural network approximation. + (** ***k***~**∆**~**+** ***κ***~**∆**~**(** ***t*** **))** ***K***~***N***~**(** ***ζ*** **)** ***e*** **(** ***t*** **)** ***, dt*** **The time derivative of the Lyapunov function** ***V*** **(** ***t*** **) can (8) be bounded as with** ***K***~***N***~**(** ***ζ*** **) = −** ***N*** **(** ***ζ*** **) is the related Nussbaum function 1**

**gain. In (8) we employed constant gains** ***k*** **,** ***k*** **, and** ***k*** **˙** ***T*** **˙ ˙ ˜ −1 ˆ˙** ***π ι*** **∆** ***V*** **(** ***t*** **) = Ψ (** ***t*** **)** ***M*** **(** ***t*** **)Ψ (** ***t*** **) + Ψ (** ***t*** **)**<sup>***T***</sup> ***M*** **(** ***t*** **)Ψ (** ***t*** **) +** ***ψ*** **(** ***t*** **)**<sup>***T***</sup> **Γ** ***ψ*** **(** ***t*** **)** ***.*** **2 alongside their time-varying analogs** ***κ***~***π***~**(** ***t*** **),** ***κ***~***ι***~**(** ***t*** **), and (14)** ***κ***~**∆**~**(** ***t*** **). This strategy enhances traditional PID controls Considering dynamics (1) and the definition by integrating adaptive gains that adjust in real-time of the generalized error Ψ (** ***t*** **), one can obtain to the system's state. To further simplify parameter ˙** ***M*** **(** ***t*** **)Ψ (** ***t*** **) =** ***C*** **(** ***t*** **)** ***q*** **˙(** ***t*** **) +** ***G*** **(** ***t*** **) −** ***τ*** **(** ***t*** **) +** ***M*** **(** ***t*** **)** ***e*** **¯(·), where** ***k***~***π***~ ***k***~***ι***~

**tuning, we establish the relationships** ***k*** **= = and**

**∆ 2*****γ***

***e*** **¯(·) =** ***q*** **¨ (** ***t*** **)+2*****γe*** **˙(** ***t*** **)+** ***γ***<sup>**2**</sup> ***e*** **(** ***t*** **). Then, utilizing the application**

***γ***<sup>**2**</sup> ***d κ***~***π***~**(** ***t*** **)** ***κ***~***ι***~**(** ***t*** **)**

***κ*** **(** ***t*** **) = = , ensuring the quadratic expression of Young's inequality, and considering Property 2, ∆ 2*****γ***

***γ***<sup>**2**</sup> **1**

**we obtain Ψ (** ***t*** **)**<sup>***T***</sup> ***C*** **(** ***t*** **)** ***q*** **˙(** ***t*** **) ≤** ***αkΨ*** **(** ***t*** **)k**<sup>**2**</sup> ***c***<sup>**2**</sup>**kq˙(** ***t*** **)k**<sup>**4**</sup> **+ ,**

**4*****α***

***S***<sup>**2**</sup> **+ 2*****γS*** **+** ***γ***<sup>**2**</sup> **is Hurwitz polynomial, where** ***S*** **is the**

***T*** **1** ***T***

**Ψ (** ***t*** **)** ***G*** **(** ***t*** **) ≤** ***αkΨ*** **(** ***t*** **)k**<sup>**2**</sup> ***g***<sup>**2**</sup> **+ , Ψ (** ***t*** **)** ***M*** **(** ***t*** **)** ***e*** **¯(·) ≤ Laplace operator. Thus, the control design primarily 4*****α***

**2 1 1** ***T***

***αkΨ*** **(** ***t*** **)k**<sup>**2**</sup> ***m***<sup>**+**</sup> **ke¯(·)k**<sup>**2**</sup> **+ , and Ψ (** ***t*** **)** ***M*** **˙ (** ***t*** **)Ψ (** ***t*** **) = depends on selecting** ***k*** **and** ***κ*** **(** ***t*** **). The PID-like control**

**∆ ∆ 4*****α*** **2** ***T*** **1**

**input is then refined to: Ψ (** ***t*** **)** ***C*** **(** ***t*** **)Ψ (** ***t*** **) ≤** ***αkΨ*** **(** ***t*** **)k**<sup>**2**</sup> ***c***<sup>**2**</sup>**kq˙(** ***t*** **)k**<sup>**2**</sup>**kΨ (** ***t*** **)k**<sup>**2**</sup> **+ , where**

**4*****α***

**Ç Z å** ***α \>*** **0 is a design parameter. Accordingly, taking into**

***t***

***d***

***u*** **(** ***t*** **) =(** ***k*** **+** ***κ*** **(** ***t*** **))** ***K*** **(** ***ζ*** **) 2*****γe*** **(** ***t*** **) +** ***γ***<sup>**2**</sup> ***e*** **(** ***ρ*** **)** ***dρ*** **+** ***e*** **(** ***t*** **) account equation (14) and applying the aforementioned ∆ ∆** ***N dt***

**0**

**inequalities, one has, = − (** ***k***~**∆**~**+** ***κ***~**∆**~**(** ***t*** **))** ***N*** **(** ***ζ*** **)Ψ** ***.*** **1**

**−1 ˙ ˙ ˜ ˆ**

***V*** **(** ***t*** **) ≤** ***αkΨ*** **(** ***t*** **)k**<sup>**2**</sup>**Λ(** ***t*** **)−Ψ (** ***t*** **)**<sup>***T***</sup> ***κu*** **(** ***t*** **)+ +Γ** ***ψ*** **(** ***t*** **)**<sup>***T***</sup> ***ψ*** **(** ***t*** **)** ***,*** **(15)**

- (9) ***α***
