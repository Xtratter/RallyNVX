# AI suite baseline / Базовые цифры набора ИИ

Measured 2026-10-22 on 2.90.0 (the Vulpes and the old offroader, four levels 1, 2, 3 and 21, the whole stage of each; times are the mean over the levels). `ai_suite` in `autopilot/suite_tests.rs`. The conditions: dry (clear sky), wet (rain), worn tyres (wear 70 %), cold tyres (5 C, air 3 C), damaged (body and engine 30 %, springs 20 %; the damage column is the start damage).

**Normal style (1)**

| car | condition | time s | edge m | flips | damage % |
|---|---|---|---|---|---|
| vulpes | dry | 318 | -0.2 | 0 | 0 |
| offroader | dry | 336 | -0.3 | 0 | 0 |
| vulpes | wet | 328 | -0.3 | 0 | 0 |
| offroader | wet | 346 | -0.3 | 0 | 0 |
| vulpes | worn tyres | 334 | -0.3 | 0 | 0 |
| offroader | worn tyres | 349 | -0.3 | 0 | 0 |
| vulpes | cold tyres | 345 | 2.4 | 0 | 0 |
| offroader | cold tyres | 356 | 3.1 | 0 | 0 |
| vulpes | damaged | 337 | -0.3 | 0 | 23 |
| offroader | damaged | 354 | -0.3 | 0 | 23 |

**Max Attack (5)**

| car | condition | time s | edge m | flips | damage % |
|---|---|---|---|---|---|
| vulpes | dry | 287 | 5.2 | 0 | 0 |
| offroader | dry | 305 | 1.7 | 0 | 0 |
| vulpes | wet | 297 | 7.1 | 0 | 0 |
| offroader | wet | 314 | 2.1 | 0 | 0 |
| vulpes | worn tyres | 305 | 9.2 | 0 | 0 |
| offroader | worn tyres | 324 | 9.2 | 0 | 0 |
| vulpes | cold tyres | 315 | 9.5 | 0 | 0 |
| offroader | cold tyres | 505 | 9.4 | 0 | 0 (2 of 4 finished) |
| vulpes | damaged | 288 | 2.3 | 0 | 23 |
| offroader | damaged | 313 | 2.4 | 0 | 23 |

What it shows: the normal style loses 3 to 5 % in wet, worn and damaged cells and goes 2 to 3 m beyond the edge on cold tyres; Max Attack is 9 % faster dry but in wet, worn and cold cells it leaves the usable ground by 7 to 9.5 m, and the offroader on cold tyres does not finish two levels out of four. The plan does not know the state of the car (the measurement of the pull and the brake is made on a clean car): this is the work of A1 and A3.

**Кратко по-русски.** Базовые цифры 2.90.0 на двух машинах, пяти условиях и четырёх уровнях. Обычный стиль теряет 3–5 % на мокром, с изношенными шинами и с повреждениями и уходит за край на 2–3 м на холодных шинах. Max Attack на сухом быстрее на 9 %, но на мокром, с изношенными и с холодными шинами уходит с пригодной земли на 7–9,5 м, а старый внедорожник на холодных шинах не финиширует на двух уровнях из четырёх. План не знает состояния машины (разгон и торможение измеряются на «чистой» машине): это работа этапов A1 и A3.
