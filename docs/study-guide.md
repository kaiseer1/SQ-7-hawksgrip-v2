# Study guide: the ideas behind an AI fleet coordinator

*Written for Basil, October 2026. Goal: you can explain every idea here in your own words, without notes, to a committee.*

**How to use this guide**
- One chapter a day. Read it once slowly, then close it and answer the check questions **out loud**.
- If you can explain a chapter to a friend who isn't in computing, you own it.
- Answers to the check questions are at the end. Look only after you've tried.
- The examples use civilian fleets (ambulances, delivery drones, rescue drones) because the ideas are universal. The same thinking applies to any system where one coordinator commands a group.

---

## Chapter 0 — The big picture

Every "smart fleet" system, whether it runs delivery drones, ambulances, taxis or rescue teams, has the same five layers:

| Layer | Question it answers | Chapter |
|---|---|---|
| **World** (simulation) | What is happening, step by step? | 1 |
| **Senses** (tracking) | What do I *believe* is happening, given noisy measurements? | 3 |
| **Matching** (assignment) | Which unit goes to which task? | 2 |
| **Judgment** (learning) | What matters most right now? Who should rest, wait, or go? | 4, 5, 6, 7 |
| **Proof and trust** | Does it really work? Can a human understand and trust it? | 8, 9 |

Hold this table in your head. Every question a committee asks will land in one of these rows.

---

## Chapter 1 — Simulation: building a world you can test in

**What a simulation is.** A program that holds the *state* of a world (positions, speeds, battery levels) and advances it in small **time steps**. Each step:
1. read the current state;
2. apply the rules (physics, decisions);
3. write the new state.

```python
# The whole idea in 6 lines: a drone flying east at 10 m/s, dt = 0.1 s
x, v, dt = 0.0, 10.0, 0.1
for step in range(100):      # 100 steps x 0.1 s = 10 seconds
    x = x + v * dt           # rule: new position = old position + speed x time
print(x)                     # 100.0 metres
```

**Why a fixed time step?** If every step is exactly the same length (say 0.05 s), the simulation behaves identically on a slow laptop and a fast PC. The *wall-clock* speed changes; the *simulated* world does not.

**Determinism and seeds.** Simulations use random numbers (sensor noise, random spawn times). Computers make "random" numbers from a starting number called a **seed**: same seed, same sequence, same run. This matters for three reasons:
- **Debugging:** a bug you saw once, you can replay exactly.
- **Science:** another researcher can reproduce your result.
- **Fair comparison:** two algorithms tested on the *same* seeds face the *same* situations.

**Separate the world from the graphics.** The simulation should not know how it is drawn. Then you can:
- run it **headless** (no window), thousands of times faster, for experiments and training;
- **record** runs and **replay** them in a viewer later;
- change the graphics without breaking the science.

**Tests.** Small programs that check one rule each, for example "a drone flying east for 10 s at 10 m/s ends 100 m east". When you change code, the tests tell you instantly if you broke something.

**Check questions**
1.1 Why does a fixed time step make results the same on different computers?
1.2 What is a seed, and give two reasons it matters for research.
1.3 Why keep the simulation separate from the graphics?

---

## Chapter 2 — The assignment problem: who goes where?

**The problem.** You have *n* units (ambulances, drones) and *m* tasks (calls, deliveries). Each pair has a **cost**, usually time to arrive. You want to assign units to tasks so the **total** cost is smallest. The costs form a table called a **cost matrix**.

**Greedy, and why it fails.** Greedy says: "pick the cheapest pair, lock it in, repeat." It feels right, but look:

| | Call 1 | Call 2 |
|---|---|---|
| Ambulance A | **1** min | 2 min |
| Ambulance B | 2 min | **10** min |

- **Greedy:** A→Call 1 (1 min, cheapest), then B is forced to Call 2 (10 min). **Total = 11.**
- **Best:** A→Call 2 (2 min), B→Call 1 (2 min). **Total = 4.**

Greedy grabbed the best *single* move and ruined the *whole*. That is the core lesson.

**The Hungarian algorithm** (Kuhn, 1955) always finds the best total. It works by repeatedly subtracting row and column minimums and covering zeros until a perfect matching of zeros appears. Its cost grows like *n³*, so a 10×10 problem is instant. It also handles **rectangular** problems (more tasks than units, or the reverse) by padding with dummy rows or columns.

**The cost is a design choice, and it is where intelligence hides.** "Time to arrive" is simple. A smarter cost might be **slack**: how much time is left before the task becomes too late. A call that will be too late in 2 minutes matters more than one that can wait 30. Choosing the cost function is a real research decision; write it down and justify it.

**Re-solving every second (rolling horizon).** In a moving world you can re-run the matching every second with fresh information. That is powerful but **myopic**: it only optimizes *now*. It never thinks "keep one unit in reserve because more calls usually come in waves." Chapter 7 is about fixing exactly that.

**Auctions and CBBA.** Instead of one central computer solving the table, units can **bid** on tasks and the highest bidder wins. Bertsekas's auction algorithm (1988) and CBBA, the Consensus-Based Bundle Algorithm (Choi, Brunet and How, 2009), are well-known versions. They work even when units can only talk to their neighbours: **decentralized**. The trade-off is that they may need many rounds of messages and are not always optimal.

**Check questions**
2.1 Using the table above, explain in one sentence why greedy is worse.
2.2 What does the Hungarian algorithm guarantee, and roughly how does its cost grow?
2.3 Why is "rolling-horizon" re-solving still myopic?
2.4 When would you prefer an auction over a central Hungarian solver?

---

## Chapter 3 — Tracking: believing the right thing from noisy measurements

**The problem.** Sensors are noisy. GPS jumps by a few metres; a camera gives a direction but not an exact distance. You want a smooth, reliable estimate of **where something is** and **how fast it is moving**.

**The alpha–beta filter**: predict, compare, correct.
1. **Predict:** `x_pred = x + v·dt` (where it should be now if it kept going).
2. **Compare:** `residual r = z − x_pred` (measurement minus prediction: the surprise).
3. **Correct:**
   - position `x = x_pred + α·r`
   - velocity `v = v + (β/dt)·r`

**Worked example** (dt = 1 s, α = 0.5, β = 0.1; start x = 0 m, v = 10 m/s)

| Step | Measurement z | Prediction | Surprise r | New x | New v |
|---|---|---|---|---|---|
| 1 | 12 | 10 | +2 | 11 | 10.2 |
| 2 | 19 | 21.2 | −2.2 | 20.1 | 9.98 |

The filter believed half of each surprise (α = 0.5) and adjusted speed only slightly (β = 0.1).

**The trade-off you must be able to explain:**
- **Large α, β:** responsive, follows sudden turns quickly, but jittery (it believes the noise).
- **Small α, β:** smooth, but slow to notice real changes.

**The Kalman filter** (Kalman, 1960) is the grown-up version. It calculates the best α and β *automatically*, at every step, from how noisy the sensor is and how unpredictable the motion is. It also tells you **how uncertain** the estimate is (the covariance). That uncertainty is valuable information for decisions: a coordinator should be careful about acting on a very uncertain track.

**Data association.** With many objects, which measurement belongs to which track? Simple systems match each measurement to the nearest predicted position (with a maximum distance, a "gate"). Research simulations sometimes assume association is perfect, but **you must state that assumption openly**.

**Fusing two sensors.** A radar is good at distance and weaker at angle; a camera is excellent at angle and poor at distance. Combining them gives an estimate better than either alone. That's **sensor fusion**.

**Check questions**
3.1 Name the three steps of the alpha–beta filter.
3.2 What happens if α is too large? Too small?
3.3 What extra information does a Kalman filter give you, and why is it useful for decisions?
3.4 Why combine a radar and a camera?

---

## Chapter 4 — Reinforcement learning: learning from consequences

**The setup.** An **agent** (the decision-maker) interacts with an **environment** (the simulation):
1. it sees an **observation** (what it currently knows);
2. it chooses an **action**;
3. the environment moves forward and returns a **reward** (a number: good or bad);
4. repeat until the **episode** ends.

The agent's goal is to maximize the **total reward over time** (the **return**), not just the next reward.

**Discount factor γ (gamma).** Future rewards count slightly less: `return = r₁ + γ·r₂ + γ²·r₃ + …`. With γ = 0.99, a reward 100 steps later still counts about 37%. Choose γ to match how far ahead consequences matter in your problem.

**Policy and value.**
- The **policy** π is the agent's behaviour: "in this situation, choose these actions with these probabilities."
- The **value** V is a prediction: "from this situation, how much total reward do I expect?"

**Why simulation is essential.** RL learns by trial and error, often *millions* of tries. You cannot crash a million real drones. You can in a simulation.

**Three classic difficulties (committees love these):**
1. **Credit assignment.** A bad outcome now may be caused by a decision made minutes ago. Which action deserves the blame?
2. **Exploration vs exploitation.** Keep doing what works, or try something new that might work better?
3. **Reward hacking.** The agent maximizes *exactly* what you reward, not what you *meant*. Famous example: an agent in a boat-racing game learned to circle forever collecting bonus points instead of finishing the race (Clark and Amodei, OpenAI, 2016). **Lesson: design and test the reward carefully, and look for exploits.**

**Action masking.** Some actions are not allowed in some situations (a drone with an empty battery cannot take a new job). A **mask** sets their probability to zero, so the agent can never choose them. Masks are also how you enforce **hard rules** that learning must never break.

**Example: a delivery-drone dispatcher**
- **Observation:** drone batteries and positions, waiting orders, the time of day.
- **Actions:** send a drone, keep it in reserve, send it to recharge.
- **Reward:** +1 per on-time delivery, −5 per late order, small cost for energy used.
- **What it might learn:** to keep a reserve before the lunch rush, which a pure re-solved matching never does.

**Check questions**
4.1 Define agent, environment, observation, action, reward.
4.2 What does γ do? What changes if γ is very small?
4.3 Explain reward hacking with an example, and one way to protect against it.
4.4 What is an action mask for?

---

## Chapter 5 — PPO: how the learning actually happens

**The intuition of policy gradients.** After trying actions, make the good ones **more likely** and the bad ones **less likely**. "Good" means *better than expected*.

**Advantage.** `advantage = what actually happened − what the value function expected`.
- Positive: this action beat expectations, so do it more.
- Negative: it disappointed, so do it less.

**GAE** (Generalized Advantage Estimation; Schulman et al., 2016) is a standard way to compute advantages that balances bias and noise using a parameter λ (often 0.95).

**The problem PPO solves.** If you change the policy too much in one update, learning can collapse: one lucky batch convinces the agent of something wrong, and it never recovers.

**PPO's fix: clipping** (Schulman et al., 2017). Compare the new policy to the old one with the ratio `ratio = π_new(action) / π_old(action)`, and don't let it move outside `[1 − ε, 1 + ε]`, usually ε = 0.2. In words: **"improve, but never more than about 20% per step."**

```
objective = average of  min( ratio × advantage,  clip(ratio, 1−ε, 1+ε) × advantage )
```

**The training loop:**
1. Run the current policy in many simulations at once and record what happened.
2. Compute returns and advantages.
3. Update the policy (with clipping) and the value function for a few passes over the data.
4. Repeat, thousands of times.

**What to watch during training:**
- **Average return:** should rise, then level off.
- **Entropy** (how random the policy is): should fall slowly. Falling too fast means it stopped exploring too early.
- **Value loss:** how well the value function predicts. If it never improves, something is wrong.

**Typical settings to know:** ε = 0.2, γ = 0.99, λ = 0.95, learning rate ≈ 3×10⁻⁴.

**Check questions**
5.1 What is the advantage, in one sentence?
5.2 What problem does clipping solve, and what does ε = 0.2 mean in plain words?
5.3 List the four steps of the PPO training loop.
5.4 Your entropy drops to almost zero after a few minutes of training. What might be wrong?

---

## Chapter 6 — Transformers and attention: handling "any number of things"

**The problem.** Ordinary neural networks need a **fixed-size** input. But a fleet changes: sometimes 3 drones and 2 orders, sometimes 7 drones and 12 orders.

**Tokens.** Describe each object (each drone, each order) as a small list of numbers called a **token**. Now the input is a *set* of tokens of any size.

**Attention** (Vaswani et al., 2017). Each token asks a question of all the others:
- its **query**: "what am I looking for?";
- each other token's **key**: "what do I offer?";
- and its **value**: "here is my information".

The token gathers information from the others, weighted by how well their keys match its query:

```
Attention(Q, K, V) = softmax( Q·Kᵀ / √d ) · V
```

`softmax` turns scores into weights that add up to 1. Dividing by `√d` keeps the scores from getting too large.

**Why it fits fleets:**
- It works for **any number** of tokens.
- It is **permutation-invariant**: the order you list the drones in doesn't change the answer, which is correct, because the order means nothing.
- **Masks** hide padding tokens (when you fill a batch to a fixed size).

**Pointer heads** (Vinyals et al., 2015). To choose *which* order a drone should take, let the drone's token "point" at one of the order tokens using attention scores. That turns choosing among a variable number of options into a probability over those options. Kool et al. (2019) used this to learn routing problems.

**Check questions**
6.1 Why can't an ordinary network handle "any number of drones"?
6.2 Explain query, key and value with an everyday analogy.
6.3 What does permutation-invariant mean, and why is it the right property here?
6.4 What does a pointer head do?

---

## Chapter 7 — Hybrid decisions: learn the judgment, keep the math

A powerful design pattern: **let learning decide *what matters*, and let a classic algorithm do *the exact matching*.**

- The learned part outputs **priorities** (which tasks are most urgent), **how many units to commit**, and **who stays in reserve or recharges**.
- The Hungarian algorithm then does the optimal matching, using those priorities in its cost.

**Why this is strong:**
1. **Safety net.** If the learned part outputs neutral priorities, you get exactly the classic method. The hybrid starts *no worse* than the baseline.
2. **Faster learning.** The network doesn't have to rediscover matching from scratch.
3. **Easier to explain.** You can say: "she raised the priority of this task because…", and the matching that follows is provably optimal for those priorities.

This family of ideas is sometimes called **learning to optimize**: machine learning guiding a classical solver.

**Check questions**
7.1 Why does the hybrid start no worse than the classic method?
7.2 Name one thing a learned policy can do that rolling-horizon matching cannot.

---

## Chapter 8 — Honest experiments: proving it works

This chapter protects your reputation. Committees and reviewers attack here first.

**Baselines.** Always compare against the strongest simple methods (greedy, Hungarian, auctions), all with the **same rules and the same information**. Beating a weak baseline proves nothing.

**Training seeds vs evaluation episodes.** RL results vary a lot between training runs (Henderson et al., 2018).
- Testing *one* trained model on 1000 episodes measures that one model.
- To claim the **method** works, train it **at least 5 times with different seeds** and report all of them.

**Held-out tests.** Evaluate on situations the agent **never trained on** (new seeds, new maps). Otherwise you may be measuring memorization.

**IQM, the interquartile mean.** Sort the scores, drop the bottom 25% and top 25%, and average the middle 50%. It resists outliers (Agarwal et al., 2021).

Example scores `[2, 50, 51, 52, 53, 54, 55, 300]`:
- Mean = 617 / 8 ≈ **77.1** (pulled up by the 300).
- IQM = mean of `[51, 52, 53, 54]` = **52.5** (the honest middle).

**Bootstrap confidence intervals.** How sure are you about that 52.5?
1. Resample your scores *with replacement* (same count) 10,000 times.
2. Compute the IQM each time.
3. The middle 95% of those 10,000 numbers is your **95% confidence interval**.

If two methods' intervals overlap a lot, you **cannot** claim one is better.

**Sensitivity sweeps.** Some numbers in any simulation are uncertain (battery performance, sensor noise). Run experiments across a **range** of each uncertain value and report **where the conclusions change**. This turns a weakness into a finding.

**Ablations.** Remove one feature at a time (for example, the reserve decision) and measure the drop. That shows *what actually matters*.

**The claims ledger.** One document listing every number the simulation uses, each with either a public source or the label **SWEPT** (uncertain, tested over a range). The paper may claim nothing that isn't backed by an experiment.

**Negative results are results.** "The learned method matched but did not beat Hungarian in calm conditions, and beat it by X in surge conditions" is an honest, publishable sentence.

**Check questions**
8.1 Why is 1000 evaluation episodes of one model not enough to claim the method works?
8.2 Compute the IQM of `[1, 9, 10, 11, 12, 100]`. (Hint: with 6 values, drop the lowest and highest 25%, which here means 1.5 values from each end. For simplicity, drop one from each end and average the middle four.)
8.3 What does overlapping confidence intervals tell you?
8.4 What is a sensitivity sweep, and why does it make a paper stronger?

---

## Chapter 9 — Explainable AI and human trust (the MIS heart of the project)

This is where **Information Systems** meets AI, and where a character like the Queen becomes a research contribution, not decoration.

**Why it matters.** People supervise automated systems. If they don't understand them, they either **over-trust** (miss failures) or **under-trust** (switch off a good system). Parasuraman and Riley (1997) call these *misuse* and *disuse*.

**Transparency levels** (Chen et al., 2014, the SAT model). A good automated agent tells the human:
1. **What** it is doing.
2. **Why** it is doing it.
3. **What will happen next**, and how sure it is.

**Anthropomorphism is double-edged.** Giving a system a personality and voice increases trust (Waytz, Heafner and Epley, 2014). That is good when the system is reliable, and dangerous when it isn't. The design goal is **calibrated trust**: trust that matches actual reliability.

**Faithfulness: the honesty test for explanations.** An explanation is *faithful* if it reflects the **real** reasons for the decision, not a nice story.
- **How to test it:** compare the stated reason ("I prioritized this because it is closest to the site") with the factors that actually drove the model's decision. They must agree.

**A character layer over a decision system** (the Queen idea, in research terms):
- The *same* decisions can be presented in two **modes** (theatrical and operational).
- Switching mode must **never change the decisions**, and a test proves it.
- The character must speak **only faithful reasons**.
- Then a study can ask: does the character help people understand and trust the system *appropriately*?

**How trust and workload are measured.**
- **Trust:** questionnaires such as the trust scale of Jian, Bisantz and Drury (2000).
- **Workload:** NASA-TLX (Hart and Staveland, 1988).
- Studies with people usually need **ethics approval** from the university first.

**Check questions**
9.1 What are misuse and disuse of automation?
9.2 Name the three levels of transparency.
9.3 What does "faithful explanation" mean, and how can you test it?
9.4 Why can a personality be risky, and what is "calibrated trust"?

---

## Chapter 10 — Why two languages: Rust and Python

- **Python** has the best tools for machine learning (PyTorch). It's fast to write, but slow at running millions of small simulation steps.
- **Rust** is as fast as C++, but its compiler prevents whole classes of crashes and memory bugs *before the program runs*.
  - **Ownership in plain words:** every piece of data has exactly one owner. When the owner is done, the data is freed automatically. Others can **borrow** it: many readers at once, or one writer, never both. The compiler checks these rules.
- **The bridge:** a Rust simulation can be called from Python (via a library called PyO3), so Python trains while Rust does the heavy stepping.
- **ONNX** is a standard file format for trained models. Train in Python, export to ONNX, run anywhere, even without Python.

**Check questions**
10.1 Why not do everything in Python?
10.2 Explain Rust's ownership and borrowing in two sentences.
10.3 What is ONNX for?

---

## The 60-second explanations (practise these out loud)

1. "My project is about one coordinator commanding a fleet. It has to decide who goes where, who waits, and who recharges, using noisy information."
2. "Greedy assignment can be badly wrong; the Hungarian algorithm finds the best total. But both only think about *now*."
3. "I use reinforcement learning so the coordinator learns *judgment*: priorities and reserves. A classic algorithm then does the exact matching."
4. "I prove it honestly: strong baselines, several training seeds, held-out tests, and confidence intervals."
5. "The character is an interface: it explains the real reasons for each decision, and changing the character never changes the decisions."

---

## Glossary

| Term | Meaning |
|---|---|
| Time step (dt) | The fixed slice of simulated time per update |
| Seed | Starting number for random generation; same seed, same run |
| Headless | Running without graphics, for speed |
| Cost matrix | Table of costs for every unit–task pair |
| Hungarian algorithm | Finds the minimum-total-cost assignment |
| Rolling horizon | Re-solving a decision repeatedly as new information arrives |
| Alpha–beta / Kalman filter | Methods to estimate position and velocity from noisy measurements |
| Covariance | The filter's measure of its own uncertainty |
| Policy (π) | The agent's behaviour: probabilities of actions per situation |
| Value (V) | Predicted future reward from a situation |
| Advantage | How much better an action did than expected |
| PPO | A stable RL algorithm that limits how much the policy changes per update |
| Entropy | How random the policy is; high means exploring |
| Action mask | Forces forbidden actions to probability zero |
| Token | A small vector describing one object |
| Attention | Lets each token gather information from the others |
| IQM | Average of the middle 50% of scores |
| Bootstrap CI | Uncertainty range from resampling the results |
| Ablation | Removing one feature to measure its contribution |
| Faithfulness | An explanation matches the model's true reasons |
| Calibrated trust | Human trust that matches the system's real reliability |

---

## Answer key

- **1.1** Each step represents the same simulated time no matter how fast the computer runs it, so the rules produce the same states.
- **1.2** The starting number for "random" generation. It enables exact replays for debugging, reproducibility for others, and fair comparisons on identical situations.
- **1.3** To run fast without graphics (experiments, training), to replay recordings, and to change visuals without touching the science.
- **2.1** Greedy locks A to the cheapest single pair (1 min), forcing B into a 10-minute trip. The total is 11 instead of 4.
- **2.2** The minimum total cost assignment; its cost grows roughly with n³.
- **2.3** It optimizes only the current moment and never plans for future demand, such as keeping a reserve.
- **2.4** When there is no reliable central computer or link, and units can only talk to neighbours.
- **3.1** Predict, compare (residual), correct.
- **3.2** Too large: jittery, follows noise. Too small: smooth but slow to react to real changes.
- **3.3** Its uncertainty (covariance). It lets decisions be cautious about unreliable tracks.
- **3.4** Each covers the other's weakness (radar range vs camera angle), giving a better estimate than either alone.
- **4.1** Agent: the decision-maker. Environment: the world it acts in. Observation: what it sees. Action: what it chooses. Reward: a number scoring the outcome.
- **4.2** It discounts future rewards. With a very small γ the agent becomes short-sighted.
- **4.3** The agent exploits the reward as written instead of as intended (the circling boat). Guard against it by testing the reward for loopholes, watching behaviour, not just scores, and using masks for hard rules.
- **4.4** To make forbidden actions impossible to choose.
- **5.1** How much better (or worse) an action turned out than the value function expected.
- **5.2** It prevents destructive, too-large policy updates. ε = 0.2 means "don't change any action's probability by more than about 20% relative to before, per update".
- **5.3** Collect experience; compute returns and advantages; update the policy (clipped) and the value function; repeat.
- **5.4** It stopped exploring too early, maybe too high a learning rate or a reward that makes one action dominate. Consider an entropy bonus or checking the reward.
- **6.1** It expects a fixed-size input; the number of drones changes.
- **6.2** Like searching a library: your query is what you want, each book's key is its title, its value is its content. You read more of the books whose titles match best.
- **6.3** Reordering the inputs doesn't change the output. Correct, because the list order of drones carries no meaning.
- **6.4** Turns attention scores into a choice among a variable number of options (e.g. which order to take).
- **7.1** With neutral priorities the hybrid reduces to the classic matching itself.
- **7.2** Plan ahead: keep units in reserve, or send them to recharge before a surge.
- **8.1** It shows one model's performance, not whether the training *method* reliably produces good models; RL varies a lot between training runs.
- **8.2** Drop 1 and 100, then average `[9, 10, 11, 12]` = **10.5** (the mean would be 23.8).
- **8.3** You can't claim a real difference between the methods.
- **8.4** Testing across a range of an uncertain value. It shows how robust the conclusions are, and is honest about uncertainty.
- **9.1** Misuse: over-reliance on automation. Disuse: rejecting automation that would help.
- **9.2** What it's doing; why; what comes next, and how sure it is.
- **9.3** The explanation reflects the true decision factors. Test it by comparing stated reasons with the model's actual influential inputs.
- **9.4** It raises trust even when the system is wrong. Calibrated trust means trust that matches actual reliability.
- **10.1** Python is too slow for millions of small simulation steps.
- **10.2** Every value has one owner and is freed when the owner finishes. Others may borrow it (many readers or one writer), and the compiler enforces it.
- **10.3** A portable format to run a trained model anywhere, without the training framework.

---

## References

- Kuhn, H. W. (1955). The Hungarian method for the assignment problem. *Naval Research Logistics Quarterly*, 2(1–2), 83–97.
- Bertsekas, D. P. (1988). The auction algorithm: A distributed relaxation method for the assignment problem. *Annals of Operations Research*, 14, 105–123.
- Choi, H.-L., Brunet, L., & How, J. P. (2009). Consensus-based decentralized auctions for robust task allocation. *IEEE Transactions on Robotics*, 25(4), 912–926.
- Kalman, R. E. (1960). A new approach to linear filtering and prediction problems. *Journal of Basic Engineering*, 82(1), 35–45.
- Schulman, J., Moritz, P., Levine, S., Jordan, M., & Abbeel, P. (2016). High-dimensional continuous control using generalized advantage estimation. *ICLR*. arXiv:1506.02438.
- Schulman, J., Wolski, F., Dhariwal, P., Radford, A., & Klimov, O. (2017). Proximal policy optimization algorithms. arXiv:1707.06347.
- Vaswani, A., et al. (2017). Attention is all you need. *NeurIPS*.
- Vinyals, O., Fortunato, M., & Jaitly, N. (2015). Pointer networks. *NeurIPS*.
- Kool, W., van Hoof, H., & Welling, M. (2019). Attention, learn to solve routing problems! *ICLR*.
- Henderson, P., et al. (2018). Deep reinforcement learning that matters. *AAAI*.
- Agarwal, R., Schwarzer, M., Castro, P. S., Courville, A., & Bellemare, M. G. (2021). Deep reinforcement learning at the edge of the statistical precipice. *NeurIPS*.
- Clark, J., & Amodei, D. (2016). Faulty reward functions in the wild. OpenAI blog.
- Parasuraman, R., & Riley, V. (1997). Humans and automation: Use, misuse, disuse, abuse. *Human Factors*, 39(2), 230–253.
- Chen, J. Y. C., et al. (2014). *Situation awareness-based agent transparency* (ARL-TR-6905). U.S. Army Research Laboratory.
- Waytz, A., Heafner, J., & Epley, N. (2014). The mind in the machine: Anthropomorphism increases trust in an autonomous vehicle. *Journal of Experimental Social Psychology*, 52, 113–117.
- Jian, J.-Y., Bisantz, A. M., & Drury, C. G. (2000). Foundations for an empirically determined scale of trust in automated systems. *International Journal of Cognitive Ergonomics*, 4(1), 53–71.
- Hart, S. G., & Staveland, L. E. (1988). Development of NASA-TLX. In *Human Mental Workload* (pp. 139–183). North-Holland.
