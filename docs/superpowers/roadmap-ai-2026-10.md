# The autopilot's roadmap: very fast on any car and in any conditions / Дорожная карта ИИ автопилота

Written 2026-10-22 after the study of racing-game AI and of the minimum-lap-time research (sources at the end). State at the start: versions up to 2.90.0 (the controller with a measured pull and brake of the car, the friction ellipse, the daring loop, the self-trained numbers, Max Attack with the rally techniques and the shoulder, a route-based net and a vision net, reward training of the net).

## The aim and how it is measured

"Very fast on any car and in any conditions" is turned into numbers, so every stage can be judged.

- **The suite** (`ai_suite`, an ignored stage test, one table): cars (Vulpes, the old offroader, later more), surfaces (gravel stage, asphalt streets), weather (dry, wet, night), tyres (new, worn, cold, wrong pressure), damage (none, some), levels (six seeds, three of them never used in a training). For every cell: the time of the whole drive, the worst excursion beyond the usable ground, flips, damage.
- **The reference:** the best time the offline optimiser (stage A2, many iterations, no time limit) finds for the same cell; the autopilot's score is `time / reference`; the aim is **at most 1.05** on the controller and **1.10** on a net, in every cell, with no flips and no excursion beyond 2 m (Max Attack: beyond the shoulder).
- **A regression rule:** a stage is done when the suite's mean score falls and no cell gets worse than 3 %.
- **Rules kept:** every stage first on the host (tests, then the suite), then a short look on the phone (asked first), then a release; heavy runs only through `~/bin/lim`; docs in English and Russian; what is not done goes into the tech debt.

## Stage A: the physics of the controller (no learning, works on any car at once)

**A0 - the suite (2.91).** Build `ai_suite` and the reference runner; record the baseline of 2.90.0 in the table; the numbers of every later stage are compared with it. Done when the baseline is in `docs/` and the suite runs in under 20 minutes on three cores.

**A1 - the measured g-g-v of the car (2.92).** Today the pull and the brake are measured (`autopilot/model.rs`) but the cornering grip is a fitted number (`lat_factor`). Add a skidpad and combined probes in the probe world: the largest lateral acceleration at 8 speeds on every surface (so the aerodynamics, the tyres and the suspension are in it), and the diagonal (braking or pulling while turning) to fit the shape of the ellipse. The speed plan then uses the measured `g_lat(v)` and the measured shape instead of the guess; `lat_factor` becomes a small safety margin. Done when: the plan's corner speed matches a measured skidpad within 3 %; a new car or a changed tuning needs no retraining; the suite shows no worse and better on the second car.

**A2 - the time-optimal line (2.93).** Replace "the line of least curvature" by a loop: speed profile, then a shift of every point of the line sideways that lowers the time (a window of a few points, steps of 0.2 m, inside the usable width with the shoulder), repeat to convergence; add the late apex on loose ground and the exit towards the inside. It also makes the offline reference of the suite. Done when the line gives at least 2 % less time than the present one on the suite without a worse excursion.

**A3 - the grip estimated while driving (2.94).** Replace the daring loop `k` (which only falls and creeps back) by an estimate of the peak lateral and forward acceleration per surface (and of the tyres' state, the wetness, the damage) from what the wheels really do (the side slip against the acceleration), smoothed; the plan is made with the estimate, with a trust margin that shrinks as the estimate settles. Add a look-ahead of the surface (water, loose gravel, wet) along the route as a sensor so a change of ground is planned, not found out. Done when the autopilot's first lap in wet, worn-tyre and cold-tyre cells is within 4 % of its tenth.

**A4 - the controller's tracking (2.95).** The remaining time lost between the plan and the drive (the car follows 93 to 99 % of the plan): a feed-forward of the steering from the plan's curvature, the throttle and brake from the plan's acceleration instead of the error alone, a speed-dependent look-ahead, the handbrake and the flick planned (a manoeuvre library with entry conditions) rather than triggered by thresholds. Done when `speed / plan` is over 0.98 on the suite.

## Stage B: a search that uses the game's own physics (the teacher gets stronger)

`Vehicle` is `Copy` and a step costs about 0.04 ms, so the autopilot can try its options on a copy of the car.

**B1 - the horizon search (2.96).** Every 0.1 to 0.2 s, in a background thread, simulate 1.5 to 2 s ahead for 6 to 12 candidate manoeuvres (brake earlier or later, a different line offset, the flick, the handbrake, a lift, a cut over the shoulder) with the controller driving the rest; keep the one with the best time and no excursion or damage; the controller follows it until the next search. There is no model error: the model is the game. Done when it beats A4 by at least 3 % on the suite and on a car and a ground it was never tuned for.

**B2 - expert iteration (2.97).** The search's choices become the labels of the nets (DAgger as in 2.85.0); the nets then copy a teacher stronger than the controller and can pass the old controller. Done when the route-based net is within 3 % of the search and the vision net within 8 % (now 22 % and 10 % behind the controller).

## Stage C: learning that generalises (any car, any conditions)

**C1 - a "passport" and a randomised world (2.98).** Inputs that describe the car (mass, power and torque curve, wheelbase, the measured g-g-v, drive layout) and the conditions (surface, wetness, tyre state); training worlds with random cars, tuning, weather, temperatures and wear. Done when one net drives an unseen car (a changed tuning counts) within 10 % of the controller.

**C2 - reinforcement learning at scale (2.99).** A headless trainer (the Rust crate on a laptop with many cores, the same physics): PPO or evolution strategies on the whole net, the reward the progress along the route per second with penalties for leaving the usable ground, damage and flips; a curriculum (easy roads and grip first, then the whole suite); the weights exported to the app (the file format already carries the net's size). Done when the net beats its teacher on the suite by at least 3 % on cars it trained on.

**C3 - the world around the car (2.100).** Rays to trees, rocks, signs and water in the sensors, junctions and the city part, and (if opponents come) cars; a net that drives with no route at all (a free ride on any road).

## Stage D: the player's side

**D1 - the settings and the explanations.** One place for the styles (Calm, Normal, Aggressive, Own, Learned, Max Attack, Net, Vision net), a plain readout of what the autopilot measured of the car (the grip, the brake, the pull), per-car nets stored with the garage, training that can be left to run while the phone charges (a warning on the battery and the heat), a score of the last drive against the reference.

**D2 - the guard rails.** Every learned thing is compared with the old one on whole stages before it replaces it (done for the reward training in 2.90.0; do it for every training); a button to go back; the phone never starts a long run without the player's tap.

## The order and what can run in parallel
A0 first (without numbers nothing can be judged). A1, A2, A3 are independent and can be done in any order; the best first is **A1** (it makes every car and tuning right at once), then A3 and A2, then A4. B1 needs A1 to A3 (it uses their plan), B2 needs B1, C1 can start after A1, C2 needs a laptop run and C1, C3 any time after A0. D follows each stage.

## Risks
- The hand-written rules overfit the six levels: the suite keeps three never-used levels and rotates them.
- Time attack styles (Max Attack, the shoulder) trade time for show: the suite scores them on the same table but has no "flair" term; flair is judged by the player.
- The phone is slow: training that takes hours belongs to the laptop; the phone runs, measures and trains only short things.
- A search on the horizon costs frames: it runs in its own thread at 5 to 10 Hz and the controller never waits for it.
- The numbers of the literature (a better line, a measured g-g) are not promises: each stage states its own threshold, and a stage that misses it is reported as it is.

## Sources
Racing-game AI: [Forza's Drivatar](https://www.gamedeveloper.com/design/how-forza-s-drivatar-actually-works), [how Forza rebuilt it](https://www.aiandgames.com/p/how-forza-rebuilt-their-drivatar), [Gran Turismo Sophy](https://www.technologyreview.com/2022/07/19/1056176/sonys-racing-ai-destroyed-its-human-competitors-by-being-nice-and-fast/), [the Nature paper](https://www.cs.utexas.edu/~pstone/Papers/bib2html/b2hd-nature22.html), [a vision-based agent](https://arxiv.org/pdf/2504.09021). Minimum lap time and adaptation: [minimum-lap-time optimisation](https://www.research.unipd.it/retrieve/e14fb26e-b9c2-3de1-e053-1705fe0ac030/2021%20-%20Minimum-lap-time%20optimization%20and%20simulation%20PP.pdf), [GripMap](https://arxiv.org/pdf/2504.12115), [online tyre adaptation (ELM)](https://arxiv.org/pdf/2303.08235), [a bank of models (LLA-MPC)](https://pith.science/paper/2505.19512), [friction from accelerations](https://www.arxiv.org/pdf/2509.15423), [a learned residual model](https://pith.science/paper/2412.03874).

---

## По-русски: коротко

**Цель в числах.** «Очень быстро на любой машине и в любых условиях» = набор `ai_suite`: машины × покрытия × погода × шины × повреждения × уровни; для каждой ячейки время всего заезда, вылеты, переворот, повреждения. Эталон — лучшее время, которое находит офлайн-оптимизатор (этап A2). Цель: **не хуже 1,05 от эталона** у контроллера и **1,10** у сети в каждой ячейке, без переворотов и вылетов дальше 2 м (у Max Attack — дальше обочины). Этап закончен, когда среднее по набору падает и ни одна ячейка не стала хуже чем на 3 %.

**Этап A (физика контроллера, без обучения, сразу для любой машины).**
- A0 (2.91): набор `ai_suite` и базовые цифры 2.90.0.
- A1 (2.92): измеренная g-g-v машины: боковое сцепление на 8 скоростях на каждом покрытии (шины, аэродинамика, подвеска) и комбинированные режимы вместо подобранного `lat_factor`; смена машины или настроек — 2 секунды измерения.
- A2 (2.93): линия по минимуму времени (цикл «профиль скорости ↔ сдвиг линии»), поздняя вершина на гравии, выход к внутренней стороне; она же даёт эталон для набора.
- A3 (2.94): сцепление оценивается на ходу (по покрытиям, шинам, мокроте, повреждениям) вместо «смелости», плюс датчик покрытия вдоль маршрута.
- A4 (2.95): точность следования плану: упреждение руля по кривизне плана, газ и тормоз по ускорению плана, планируемые приёмы (разворот, ручник) вместо порогов.

**Этап B (поиск на физике самой игры).** `Vehicle` копируется, шаг ≈ 0,04 мс: B1 (2.96) каждые 0,1–0,2 с в фоне проигрывается 1,5–2 с вперёд по 6–12 манёврам, выбирается лучший по времени без вылета; ошибки модели нет, модель — сама игра. B2 (2.97): выбор поиска становится метками для сетей (DAgger), сеть копирует учителя сильнее контроллера.

**Этап C (обучение, которое обобщается).** C1 (2.98): «паспорт машины» (масса, мощность, колёсная база, g-g-v) и условий на входе сети, обучающие миры со случайными машинами, погодой, износом. C2 (2.99): RL на ноутбуке с несколькими ядрами (PPO или ES), награда за прогресс по маршруту, куррикулум, веса в приложение. C3 (2.100): лучи до деревьев, камней, знаков, перекрёстки города, езда вообще без маршрута.

**Этап D (сторона игрока).** Одно место для стилей, понятная выдача измеренного (сцепление, тормоз, тяга), сети на каждую машину в гараже, обучение на зарядке с предупреждением о батарее и нагреве, счёт последнего заезда против эталона; любое обученное сравнивается со старым на целых этапах перед заменой (в 2.90.0 сделано для обучения по награде).

**Порядок.** Сначала A0 (без чисел ничего не оценить). A1, A2, A3 независимы, начать лучше с **A1**, потом A3 и A2, потом A4. B1 нужны A1–A3, B2 нужен B1, C1 можно после A1, C2 требует ноутбука и C1, C3 в любое время после A0.

**Риски.** Правила переобучаются на шести уровнях: в наборе три уровня, которых никогда не было в обучении, они меняются. Стили «на показ» (Max Attack) оцениваются той же таблицей без «эффектности». Длинное обучение — на ноутбуке, телефон только измеряет и учит коротко. Поиск на горизонте идёт в своём потоке (5–10 Гц) и контроллер его не ждёт. Цифры из статей — не обещания: у каждого этапа свой порог, и если этап его не взял, это будет сказано как есть.


## Status log / Ход работ
- **A0 (2.91.0), A1 (2.91.0), A3 (2.92.0): done**, numbers in `docs/ai-suite-baseline.md`.
- **A2 (2.93.0): measured, no gain.** The blend of the line of least curvature with the shortest line and a later apex, chosen bend by bend by the time of the whole profile, found 0.0 s on a stage (300.6 s before and after): the line of least curvature is already the optimum of this plan. The threshold of the roadmap (2 % lower time) was not met, the code was dropped. The lap-time reference will come from the horizon search (B1).
- **A4 (2.93.0), first part:** the tracking is good sideways (rms 0.41 m in the normal style); the throttle response was weak and is stronger now (cold and worn cells 1 to 4 % faster, the clean case the same). What is left of the clean-case time is in the plan's own limits.
- **Next:** B1, the horizon search with the game's own physics, which can choose per situation how late to brake and how hard to take a bend, and can give stronger labels to the nets (B2).

**По-русски:** A0, A1, A3 выполнены. A2 измерен, выигрыша нет: линия наименьшей кривизны уже оптимальна для этого плана (300,6 с до и после), порог 2 % не достигнут, код убран. A4, первая часть: боковое следование хорошее, газ был слабым и стал строже (холодные и изношенные ячейки на 1–4 % быстрее, чистый случай тот же). Дальше B1: поиск на горизонте на физике самой игры.
