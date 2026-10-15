# Vehicle parameters

[Русский](#параметры-техники) · **English**

All numbers of the vehicles live in one file, `rust/vehicles.json`, which is embedded into the app at build time. Edit it and rebuild, or on desktop try values without rebuilding: `RALLYNVX_VEHICLES=/path/to/vehicles.json rallynvx-desktop --game`. An invalid file is ignored (the log says why) and the embedded one is used. Collision shapes and the 3D models of the vehicles are still in the code.

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
| `aero` | optional `{cl_front, cl_rear, ride_ref}`: base axle lift coefficients (negative = downforce) and the reference ride height in m |
| `upgrade_cost` | base price of one upgrade level; levels cost 1x, 2x and 4x of it (optional, default 300) |
| `handling.steer_max` | steering angle limit, rad |
| `handling.steer_speed_ref` | m/s; the steering angle shrinks as `1 / (1 + speed / ref)` |
| `handling.steer_rate` | how fast the wheels turn, rad/s |
| `handling.lateral_stiffness` | tyre side stiffness (1/s; higher = less sliding); the tarmac value, scaled by the surface |
| `handling.rolling_resistance` | N·s/m per wheel; the tarmac value, scaled by the surface |
| `handling.aero_drag` | air drag coefficient (force = coefficient × speed²) |
| `handling.angular_damping` | rotation damping, 1/s |
| `handling.handbrake_grip` | rear grip factor with the handbrake (0..1; lower = more drift) |
| `handling.reverse_speed_factor` | top reverse speed as a fraction of `max_speed` |

`look` (optional, per car): `body_color`, `accent`, `glass` (RGB 0..1, linear) and `wheel_width` (m). They only change the generated model, not the handling.

The `vulpes` car follows the data of the real rally car it is modelled on: mass 1230 kg, track 1.534 m, wheelbase about 2.45 m, length about 4 m, 300 hp / about 550 N·m with full-time four-wheel drive. `engine_force` (18000 N at standstill, fading to zero at `max_speed`) is tuned to a 0-100 km/h time of about 3.9 s and `max_speed` (52 m/s, 187 km/h) matches the gravel gearing.

## Lamps of a car model

The lamps of a car with a mesh asset (`.rcar`, material 5 = emissive) are found on the mesh itself, so they sit where the designer put them and have the mesh's shape (`rcar::lamp_points`, `carlights::lamps`): colour the lamp meshes by class, the converter keeps the colours. Headlight units: cream or white (255, 242, 204) in front; tail units: red (242, 13, 8) at the rear; reverse lamps: white (255, 255, 255) at the rear; indicators: orange (255, 140, 26). The outermost unit of at least a few vertices on the right-hand side is the car's lamp (a centre brake light or a fog lamp sliver does not count); the left-hand side is its mirror image. At night the shader (material 7, `MAT_CARLAMP`) lights each class by the car's state: headlights on, braking, reversing; the low beam starts at the two headlight units. The front unit is split in two by position: the outboard corner piece (cream vertices within 6 cm of the outermost x, at least 6 of them) becomes the position lamp (material 8, `MAT_CARPOS`, it burns with the low beam), the rest is the main lens; a mesh without a corner piece has its position lamp at the headlight. Indicators are found by colour but not lit yet (turn signals are tech debt). Boxy generated cars keep their glowing boxes.

## Surfaces (`rust/surfaces.json`)

The grip of every surface and the loose gravel layer live in a second file, `rust/surfaces.json`, embedded at build time. Per surface (`asphalt`, `dirt`, `grass`, `sand`, `rock`, `water`): `mu` (multiplies the car's `grip`), `lateral` (multiplies `handling.lateral_stiffness`), `rolling` (multiplies `handling.rolling_resistance`); `gravel` has `mu_loose` (untouched loose gravel) and `mu_packed` (the swept line) instead of `mu`; `loose_layer.wear_per_metre` is how fast a wheel packs the loose gravel (0.5 = half a cell's looseness per metre travelled). The car values are the tarmac values, so asphalt is 1.0 everywhere. Each surface also has a `dust` object: `color` (RGB 0..1), `roll` (0..1, dust while rolling), `slide` (0..2, dust while sliding or spinning), `size` (m) and `life` (s); the top-level `dust` holds `cap_normal` / `cap_lots` (the particle caps of the Normal and Lots settings) and the top-level `wind` holds `speed` (m/s, the strongest mean wind) and `gust` (0..1, how much the gusts swing it). Without a rebuild: on Android put your own `surfaces.json` next to `profile.json` (`/data/data/dev.starinin.rallynvx/files/`, copy it with root); it is read at the start of a drive; on desktop point `RALLYNVX_SURFACES=/path/to/surfaces.json` at it. A file that does not parse or is out of range is ignored (the log says why) and the embedded numbers stay.

---

# Параметры техники

Все числа техники лежат в одном файле `rust/vehicles.json`, он встраивается в приложение при сборке. Измените его и пересоберите, а на десктопе можно пробовать значения без пересборки: `RALLYNVX_VEHICLES=/путь/к/vehicles.json rallynvx-desktop --game`. Неверный файл игнорируется (причина пишется в лог), берётся встроенный. Формы столкновений и 3D-модели техники пока остаются в коде.

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
| `aero` | необязательно `{cl_front, cl_rear, ride_ref}`: базовые коэффициенты подъёмной силы осей (минус = прижим) и опорная высота кузова в м |
| `upgrade_cost` | базовая цена одного уровня улучшения; уровни стоят 1x, 2x и 4x от неё (необязательно, по умолчанию 300) |
| `handling.steer_max` | предел угла руля, рад |
| `handling.steer_speed_ref` | м/с; угол руля уменьшается как `1 / (1 + скорость / ref)` |
| `handling.steer_rate` | скорость поворота колёс, рад/с |
| `handling.lateral_stiffness` | боковая жёсткость шин (1/с; больше — меньше скольжения); значение для асфальта, масштабируется покрытием |
| `handling.rolling_resistance` | Н·с/м на колесо; значение для асфальта, масштабируется покрытием |
| `handling.aero_drag` | коэффициент сопротивления воздуха (сила = коэффициент × скорость²) |
| `handling.angular_damping` | демпфирование вращения, 1/с |
| `handling.handbrake_grip` | доля сцепления задних колёс с ручником (0..1; меньше — сильнее занос) |
| `handling.reverse_speed_factor` | максимальная скорость заднего хода как доля `max_speed` |

`look` (необязательно, у каждой машины): `body_color`, `accent`, `glass` (RGB 0..1, линейные) и `wheel_width` (м). Меняют только сгенерированную модель, не управляемость.

Машина `vulpes` следует данным настоящего раллийного автомобиля, по которому сделана модель: масса 1230 кг, колея 1,534 м, колёсная база около 2,45 м, длина около 4 м, 300 л.с. / около 550 Н·м, постоянный полный привод. `engine_force` (18000 Н с места, спадает до нуля к `max_speed`) подобрана под разгон до 100 км/ч за 3,9 с, а `max_speed` (52 м/с, 187 км/ч) соответствует гравийной передаточной паре.

## Огни модели машины

Огни машины с моделью (`.rcar`, материал 5 = светящийся) ищутся на самой модели, поэтому стоят там, где их поставил дизайнер, и имеют форму модели (`rcar::lamp_points`, `carlights::lamps`): раскрасьте огни по классам, конвертер сохраняет цвета. Фары: кремовый или белый (255, 242, 204) спереди; задние огни: красный (242, 13, 8) сзади; фонари заднего хода: белый (255, 255, 255) сзади; поворотники: оранжевый (255, 140, 26). Огнём машины считается самый внешний блок из нескольких вершин на правой стороне (центральный стоп-сигнал или полоска противотуманки не в счёт), левая сторона зеркальна. Ночью шейдер (материал 7, `MAT_CARLAMP`) зажигает каждый класс по состоянию машины: фары, тормоз, задний ход; конус света начинается у блоков фар. Угловатые генерируемые машины сохраняют светящиеся коробки.

## Покрытия (`rust/surfaces.json`)

Сцепление каждого покрытия и рыхлый слой гравия лежат во втором файле, `rust/surfaces.json`, он встраивается при сборке. Для каждого покрытия (`asphalt`, `dirt`, `grass`, `sand`, `rock`, `water`): `mu` (умножает `grip` машины), `lateral` (умножает `handling.lateral_stiffness`), `rolling` (умножает `handling.rolling_resistance`); у `gravel` вместо `mu` есть `mu_loose` (нетронутый рыхлый гравий) и `mu_packed` (утрамбованная линия); `loose_layer.wear_per_metre` — как быстро колесо утрамбовывает рыхлый гравий (0,5 = половина рыхлости ячейки на метр пути). Значения машин — это значения для асфальта, поэтому у асфальта везде 1,0. У каждого покрытия есть также объект `dust`: `color` (RGB 0..1), `roll` (0..1, пыль при качении), `slide` (0..2, пыль при скольжении и пробуксовке), `size` (м) и `life` (с); в `dust` верхнего уровня лежат `cap_normal` / `cap_lots` (лимиты частиц настроек «Обычная» и «Много»), а в `wind` верхнего уровня `speed` (м/с, самый сильный средний ветер) и `gust` (0..1, насколько порывы его качают). Без пересборки: на Android положите свой `surfaces.json` рядом с `profile.json` (`/data/data/dev.starinin.rallynvx/files/`, копируйте с root), он читается в начале поездки; на десктопе укажите на него `RALLYNVX_SURFACES=/путь/к/surfaces.json`. Файл, который не разбирается или выходит за допустимые пределы, игнорируется (причина в логе), остаются встроенные числа.


## Livery texture (`RCA2`)

The Peugeot's body is textured: `rally_car.rcar` (format `RCA2`: the vertex has a `u16x2` UV after the colour, the sentinel `0xFFFF, 0xFFFF` means "no texture, use the vertex colour"; the old `RCAR` files still load) and `rally_car_livery.jpg`, a 2048 x 2048 atlas that `tools/carconv` builds from the model's base-colour textures: the big livery texture takes the whole width (scaled to 2048), the small ones (plastic, chrome, details) are capped at 256 pixels, textures that few vertices use keep a baked colour. The game decodes the JPEG once (`zune-jpeg`), builds the mip chain and `fs_model` multiplies the albedo by the atlas where the vertex has a UV; lamps, glass and the wheels keep vertex colours. A missing or broken JPEG leaves the vertex colours.

To convert a new model: `cd tools/carconv && RUSTFLAGS="-C link-arg=-lc++" cargo build --release` (Termux), then `carconv in.glb out.rcar 12000 1500 livery.jpg` and copy both files to `rust/assets`.

## Текстура ливреи (`RCA2`)

Кузов Peugeot текстурирован: `rally_car.rcar` (формат `RCA2`: после цвета у вершины идёт `u16x2` UV, значение `0xFFFF, 0xFFFF` означает «без текстуры, берём цвет вершины»; старые файлы `RCAR` по-прежнему читаются) и `rally_car_livery.jpg` — атлас 2048 x 2048, который `tools/carconv` собирает из базовых цветовых текстур модели: большая текстура ливреи занимает всю ширину (масштаб до 2048), мелкие (пластик, хром, детали) ограничены 256 пикселями, текстуры, которыми пользуются немногие вершины, остаются запечённым цветом. Игра один раз декодирует JPEG (`zune-jpeg`), строит цепочку мипов, а `fs_model` умножает альбедо на атлас там, где у вершины есть UV; фонари, стёкла и колёса остаются на цветах вершин. Если JPEG нет или он повреждён, остаются цвета вершин.

Чтобы конвертировать новую модель: `cd tools/carconv && RUSTFLAGS="-C link-arg=-lc++" cargo build --release` (Termux), затем `carconv in.glb out.rcar 12000 1500 livery.jpg` и скопировать оба файла в `rust/assets`.


## Wheels, brake discs and calipers (`RCA3`)

`rally_car.rcar` is an `RCA3` file: a `u32` mesh count (2 to 4) and the meshes in the order body, wheel (tyre and rim), brake disc, brake caliper, all with 32-byte vertices and UVs into the livery atlas (`rally_car_livery.jpg`, now 2048 x 2560: the livery takes the whole width, the lamps, wheels and details the rest). `RCAR` and `RCA2` files still load (no disc or caliper). The wheel is drawn four times (the right-hand side turned by 180 degrees); the disc (class 13, `MAT_CARDISC`) turns with the wheel and glows orange-red by the wheel's brake heat (`Vehicle::brake_heat`, 0..1: it grows with the braking power, the front brakes more, and cools with time and speed; purely visual, strongest in the dark); the caliper follows the steering and the suspension but does not spin. The mud flaps are part of the body mesh. `tools/carconv` takes `tyre_tarmac_lf`, `x0_wheel_fl_5radios`, `x0_disc_fl` and `x0_hub_caliper_fl` of the source model, centres them on the same point and packs their textures into the atlas (wheel textures at 512 pixels at most).

## Колёса, тормозные диски и суппорты (`RCA3`)

`rally_car.rcar` — файл `RCA3`: счётчик мешей `u32` (от 2 до 4) и меши в порядке: кузов, колесо (шина с диском), тормозной диск, тормозной суппорт; вершины по 32 байта с UV в атласе ливреи (`rally_car_livery.jpg`, теперь 2048 x 2560: ливрея занимает всю ширину, фонари, колёса и детали — остальное). Файлы `RCAR` и `RCA2` по-прежнему читаются (без диска и суппорта). Колесо рисуется четыре раза (правая сторона повёрнута на 180 градусов); диск (класс 13, `MAT_CARDISC`) вращается с колесом и светится оранжево-красным по нагреву тормозов колеса (`Vehicle::brake_heat`, 0..1: растёт с мощностью торможения, у передних сильнее, и остывает со временем и скоростью; только визуально, ярче всего в темноте); суппорт следует за рулём и подвеской, но не вращается. Брызговики — часть меша кузова. `tools/carconv` берёт `tyre_tarmac_lf`, `x0_wheel_fl_5radios`, `x0_disc_fl` и `x0_hub_caliper_fl` исходной модели, центрирует их в одной точке и упаковывает их текстуры в атлас (текстуры колеса не больше 512 пикселей).
