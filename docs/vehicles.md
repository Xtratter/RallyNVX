# Vehicle parameters

[Русский](#параметры-техники) · **English**

All numbers of the vehicles live in one file, `rust/vehicles.json`, which is embedded into the app at build time. Edit it and rebuild, or on desktop try values without rebuilding: `RAYTRACE_VEHICLES=/path/to/vehicles.json raytrace-desktop --game`. An invalid file is ignored (the log says why) and the embedded one is used. Collision shapes and the 3D models of the vehicles are still in the code.

## `cars.<id>` (the id `offroader` is used by the game; add more entries for new cars)

| Field | Meaning |
|---|---|
| `mass` | kg; the moments of inertia are computed from it and `size` |
| `size.width/height/length` | m, the body box used for inertia |
| `wheels` | four `[x, y, z]` suspension mounts in the body frame: front left, front right, rear left, rear right (+z forward, +x to the car's left). The centre of mass is at the body origin, so the mount height `y` sets how low it sits: the ground is `rest + radius - y` below it. Rollover needs `half_track / that height` above `grip`, otherwise the car rolls before it slides (offroader: 0.95 / 0.70 = 1.36 against 1.1) |
| `suspension.rest` | spring rest length, m |
| `suspension.radius` | wheel radius, m |
| `suspension.spring`, `damper` | N/m and N·s/m per wheel |
| `suspension.max_force_factor` | cap of one spring force, in car weights per wheel |
| `engine_force` | total drive force, N (split over four wheels) |
| `brake_force` | total brake force, N |
| `max_speed` | m/s, drive force fades to zero here |
| `grip` | tyre friction coefficient |
| `water_drag` | extra drag in water, 1/s with all four wheels wet (optional, default 0) |
| `model` | generated body style: `suv` (default) or `rally` (optional) |
| `price` | garage price in money, 0 = owned from the start (optional, default 0) |
| `upgrade_cost` | base price of one upgrade level; levels cost 1x, 2x and 4x of it (optional, default 300) |
| `handling.steer_max` | steering angle limit, rad |
| `handling.steer_speed_ref` | m/s; the steering angle shrinks as `1 / (1 + speed / ref)` |
| `handling.steer_rate` | how fast the wheels turn, rad/s |
| `handling.lateral_stiffness` | tyre side stiffness (1/s; higher = less sliding) |
| `handling.rolling_resistance` | N·s/m per wheel |
| `handling.aero_drag` | air drag coefficient (force = coefficient × speed²) |
| `handling.angular_damping` | rotation damping, 1/s |
| `handling.handbrake_grip` | rear grip factor with the handbrake (0..1; lower = more drift) |
| `handling.reverse_speed_factor` | top reverse speed as a fraction of `max_speed` |

`look` (optional, per car and aircraft): `body_color`, `accent`, `glass` (RGB 0..1, linear) and for cars `wheel_width` (m). They only change the generated model, not the handling.

## `aircraft.heli` and `aircraft.drone`

`max_tilt_deg` (largest pitch/roll), `tilt_lag` (s), `max_speed` (m/s), `drag` (1/s), `vertical_speed` (m/s at full lift stick), `vertical_accel` (m/s²), `yaw_rate` (rad/s), `yaw_lag` (s), `radius` (collision radius, m).

## `flight` (rules for all aircraft)

`ceiling` (m), `soft_speed` (highest soft touchdown speed into the ground, m/s), `soft_tilt_deg` (largest soft tilt), `ground_damp` (ground friction, 1/s).

## `sling`

`rope` (rest length, m), `k` (rope spring), `c` (rope damper), `drag` (cargo air drag).

---

# Параметры техники

Все числа техники лежат в одном файле `rust/vehicles.json`, он встраивается в приложение при сборке. Измените его и пересоберите, а на десктопе можно пробовать значения без пересборки: `RAYTRACE_VEHICLES=/путь/к/vehicles.json raytrace-desktop --game`. Неверный файл игнорируется (причина пишется в лог), берётся встроенный. Формы столкновений и 3D-модели техники пока остаются в коде.

## `cars.<id>` (игра использует id `offroader`; для новых машин добавляйте записи)

| Поле | Смысл |
|---|---|
| `mass` | кг; по ней и по `size` считаются моменты инерции |
| `size.width/height/length` | м, габаритный бокс для инерции |
| `wheels` | четыре точки подвески `[x, y, z]` в системе кузова: переднее левое, переднее правое, заднее левое, заднее правое (+z вперёд, +x влево от машины). Центр масс находится в начале координат кузова, поэтому высота точки `y` задаёт, насколько он низко: земля ниже него на `rest + radius - y`. Чтобы машина скользила раньше, чем опрокинется, `полуколея / эта высота` должна быть больше `grip` (внедорожник: 0,95 / 0,70 = 1,36 против 1,1) |
| `suspension.rest` | длина пружины в покое, м |
| `suspension.radius` | радиус колеса, м |
| `suspension.spring`, `damper` | Н/м и Н·с/м на колесо |
| `suspension.max_force_factor` | предел силы одной пружины в весах машины на колесо |
| `engine_force` | суммарная тяга, Н (делится на четыре колеса) |
| `brake_force` | суммарная тормозная сила, Н |
| `max_speed` | м/с, на этой скорости тяга сходит на нуль |
| `grip` | коэффициент трения шин |
| `water_drag` | дополнительное сопротивление в воде, 1/с при четырёх мокрых колёсах (необязательно, по умолчанию 0) |
| `model` | стиль генерируемого кузова: `suv` (по умолчанию) или `rally` (необязательно) |
| `price` | цена в гараже, 0 = есть с самого начала (необязательно, по умолчанию 0) |
| `upgrade_cost` | базовая цена одного уровня улучшения; уровни стоят 1x, 2x и 4x от неё (необязательно, по умолчанию 300) |
| `handling.steer_max` | предел угла руля, рад |
| `handling.steer_speed_ref` | м/с; угол руля уменьшается как `1 / (1 + скорость / ref)` |
| `handling.steer_rate` | скорость поворота колёс, рад/с |
| `handling.lateral_stiffness` | боковая жёсткость шин (1/с; больше — меньше скольжения) |
| `handling.rolling_resistance` | Н·с/м на колесо |
| `handling.aero_drag` | коэффициент сопротивления воздуха (сила = коэффициент × скорость²) |
| `handling.angular_damping` | демпфирование вращения, 1/с |
| `handling.handbrake_grip` | доля сцепления задних колёс с ручником (0..1; меньше — сильнее занос) |
| `handling.reverse_speed_factor` | максимальная скорость заднего хода как доля `max_speed` |

`look` (необязательно, у машин и аппаратов): `body_color`, `accent`, `glass` (RGB 0..1, линейные) и у машин `wheel_width` (м). Меняют только сгенерированную модель, не управляемость.

## `aircraft.heli` и `aircraft.drone`

`max_tilt_deg` (наибольший тангаж/крен), `tilt_lag` (с), `max_speed` (м/с), `drag` (1/с), `vertical_speed` (м/с при полном стике высоты), `vertical_accel` (м/с²), `yaw_rate` (рад/с), `yaw_lag` (с), `radius` (радиус столкновений, м).

## `flight` (правила для всех аппаратов)

`ceiling` (м), `soft_speed` (наибольшая мягкая скорость касания в землю, м/с), `soft_tilt_deg` (наибольший мягкий наклон), `ground_damp` (трение о землю, 1/с).

## `sling`

`rope` (длина троса в покое, м), `k` (пружина троса), `c` (демпфер троса), `drag` (сопротивление воздуха грузу).
