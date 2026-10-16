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


## Wheel sets (`RCA3`, six meshes)

`rally_car.rcar` may carry six meshes: body, wheel (tarmac), disc, caliper, wheel (gravel), wheel (snow). `VehicleSpec.wheel_set` (0 tarmac, 1 gravel, 2 snow; the garage's ninth level, free, saved per car in the profile as `wheel_set`) picks the wheel mesh (`CarModel::wheel_for`; a missing set shows the tarmac wheel; `car_key` includes the set so the GPU rebuilds the wheel when it changes). The sets are `tyre_tarmac_lf` + `x0_wheel_fl_5radios`, `tyre_gravel_lf` + `x0_wheel_fl_gravel`, `snow_tire_lf` + `rim_snow_lf`; the source hides the gravel and snow parts with a 0.01 scale of their parent, `tools/carconv` scales them back about the hub. The atlas is 2048 x 2688; the disc and the caliper are shared by all sets. The set changes only the look for now; its grip effect is a later stage.

## Комплекты колёс (`RCA3`, шесть мешей)

`rally_car.rcar` может нести шесть мешей: кузов, колесо (асфальт), диск, суппорт, колесо (гравий), колесо (снег). `VehicleSpec.wheel_set` (0 асфальт, 1 гравий, 2 снег; девятая настройка гаража, бесплатная, сохраняется для каждой машины в профиле как `wheel_set`) выбирает меш колеса (`CarModel::wheel_for`; отсутствующий комплект показывает асфальтовое колесо; `car_key` включает комплект, поэтому GPU пересобирает колесо при смене). Комплекты: `tyre_tarmac_lf` + `x0_wheel_fl_5radios`, `tyre_gravel_lf` + `x0_wheel_fl_gravel`, `snow_tire_lf` + `rim_snow_lf`; в исходнике гравийные и зимние части скрыты масштабом 0,01 у родителя, `tools/carconv` возвращает им масштаб относительно ступицы. Атлас 2048 x 2688; диск и суппорт общие для всех комплектов. Пока комплект меняет только вид, влияние на сцепление будет позже.


## Suspension (class 14)

The body mesh of the Peugeot carries the suspension parts of all four corners (`x0_wishbone_*`, `x0_suspension*_*` of the source model, about 2 300 triangles, a 256-pixel tile in the atlas) as class 14 (`MAT_CARSUSP`). Per vertex: the red channel is the weight (1 at the hub, 0 at the part's far end: `1 - distance to the hub / the part's largest distance`), the green channel the corner (0 front-left (+x), 1 front-right, 2 rear-left, 3 rear-right); `vs_model` lifts the vertex by `Uni.susp[corner] * weight` in the body frame, where `susp` is `vehicle_models::susp_offsets`: each wheel's compression minus the static compression, in metres, clamped to +-0.25. So the outer ends of the arms and struts follow the hub (jumps, bumps, roll) and the inner ends stay on the body. Convert with `carconv in.glb out.rcar 16500 2800 livery.jpg` (the body budget includes the suspension).

## Подвеска (класс 14)

Меш кузова Peugeot несёт детали подвески всех четырёх углов (`x0_wishbone_*`, `x0_suspension*_*` исходной модели, около 2 300 треугольников, плитка 256 пикселей в атласе) как класс 14 (`MAT_CARSUSP`). У каждой вершины красный канал — вес (1 у ступицы, 0 на дальнем конце детали: `1 - расстояние до ступицы / наибольшее расстояние детали`), зелёный — угол (0 переднее левое (+x), 1 переднее правое, 2 заднее левое, 3 заднее правое); `vs_model` поднимает вершину на `Uni.susp[угол] * вес` в системе кузова, где `susp` — это `vehicle_models::susp_offsets`: сжатие каждого колеса минус статическое сжатие, в метрах, не больше +-0,25. Поэтому внешние концы рычагов и стоек следуют за ступицей (прыжки, неровности, крен), а внутренние остаются на кузове. Конвертация: `carconv in.glb out.rcar 16500 2800 livery.jpg` (бюджет кузова включает подвеску).


## Glass and interior (`RCA3`, eight meshes)

`rally_car.rcar` carries up to eight meshes: body, wheel (tarmac), disc, caliper, wheel (gravel), wheel (snow), **glass** (index 6) and **interior** (index 7). The body has no glass of its own. The glass mesh is the windows (`x0_window_*`, vertex red 0) and the clear lamp covers (the front headlight and turn covers, the rear lamp cover, the roof light bar's cover; vertex red 255); it is drawn after everything opaque with alpha blending, no depth write and no culling (`fs_glass`: a dark tint, the sky reflected by Fresnel, a sun glint; alpha `0.2 + 0.3 fresnel` near the car, rising to 0.95 between 4 and 8 m of camera distance so a far car has solid dark windows; a lamp cover is a quarter as strong). The interior mesh (class 15, about 25 000 triangles: the cabin `int_*` parts with the dashboard, seats, steering wheel and door panels, and the driver) is textured from its own 1024 x 1024 atlas `rally_car_interior.jpg` (bindings 10 and 11), lit as plastic in the shade of the roof (a factor, no shadow map) and drawn only when the camera is within 8 m of the car. The dust's signed distance field is built from the body and the glass, so the window holes do not let dust into the cabin. Convert with `carconv in.glb out.rcar 16500 2800 livery.jpg interior.jpg [cabin_tris driver_tris]` (defaults 13 000 and 3 000).

## Стёкла и салон (`RCA3`, восемь мешей)

`rally_car.rcar` несёт до восьми мешей: кузов, колесо (асфальт), диск, суппорт, колесо (гравий), колесо (снег), **стёкла** (индекс 6) и **салон** (индекс 7). У кузова своих стёкол больше нет. Меш стёкол — окна (`x0_window_*`, красный канал вершины 0) и прозрачные колпаки фонарей (колпаки фар и поворотников спереди, колпак заднего фонаря, колпак светодиодной панели на крыше; красный 255); он рисуется после всего непрозрачного с альфа-смешиванием, без записи глубины и без отсечения (`fs_glass`: тёмный оттенок, отражение неба по Френелю, блик солнца; альфа `0,2 + 0,3 френель` рядом с машиной, растёт до 0,95 между 9 и 16 м расстояния до камеры, поэтому у далёкой машины окна сплошные тёмные; у колпака фонаря она в четыре раза слабее). Меш салона (класс 15, около 25 000 треугольников: детали кабины `int_*` с панелью, сиденьями, рулём и обивкой дверей, и водитель) текстурируется из собственного атласа 1024 x 1024 `rally_car_interior.jpg` (привязки 10 и 11), освещается как пластик в тени крыши (коэффициент, без карты теней) и рисуется, только когда камера ближе 16 м к машине. Поле расстояний пыли строится из кузова и стёкол, поэтому проёмы окон не пускают пыль в салон. Конвертация: `carconv in.glb out.rcar 16500 2800 livery.jpg interior.jpg [треуг_салона треуг_водителя]` (по умолчанию 13 000 и 3 000).


## Cockpit (`RCA3`, eleven meshes)

The `RCA3` file may carry eleven meshes: ..., glass (6), interior (7, the cabin without the steering wheel and the driver), **steering wheel** (8, in the body frame), **steering pivot** (9: a two-vertex mesh whose first vertex has `pos` = the wheel's centre and `nrm` = its axis towards the driver; `tools/carconv` takes the centroid and the smallest principal axis of the wheel's vertices) and the **driver** (10). `build_rally` shifts the wheel mesh by `-pivot` and `CarModel.steering` keeps the pivot and axis; the renderer turns the wheel about the axis by `steering_wheel_angle(steer)` (`-steer * 5.5`, at most 3.8 rad). The driver is not drawn when the camera is within 1.2 m of the car's centre (the cockpit view). `vehicles.json` -> `cameras.cockpit` gives the driver's eyes in metres (`[x to the left, y above the road, z ahead of the centre]`; a car without it uses the bonnet's spot); the near plane is 0.08 m when the camera is within 2.5 m of the car. Convert with `carconv in.glb out.rcar 16500 2800 livery.jpg interior.jpg [cabin driver steering]` (defaults 13 000, 3 000, 2 500 triangles).

## Кабина (`RCA3`, одиннадцать мешей)

Файл `RCA3` может нести одиннадцать мешей: ..., стёкла (6), салон (7, кабина без руля и водителя), **руль** (8, в системе кузова), **ось руля** (9: меш из двух вершин, у первой `pos` = центр руля, `nrm` = его ось к водителю; `tools/carconv` берёт центроид и наименьшую главную ось вершин руля) и **водитель** (10). `build_rally` сдвигает меш руля на `-pivot`, а `CarModel.steering` хранит центр и ось; рендер поворачивает руль вокруг оси на `steering_wheel_angle(steer)` (`-steer * 5,5`, не больше 3,8 рад). Водитель не рисуется, когда камера ближе 1,2 м к центру машины (вид из кабины). `vehicles.json` -> `cameras.cockpit` задаёт глаза водителя в метрах (`[x влево, y над дорогой, z вперёд от центра]`; у машины без него берётся место капота); ближняя плоскость 0,08 м, когда камера ближе 2,5 м к машине. Конвертация: `carconv in.glb out.rcar 16500 2800 livery.jpg interior.jpg [салон водитель руль]` (по умолчанию 13 000, 3 000, 2 500 треугольников).

The file may carry fourteen meshes: after the driver (10) come the driver's-side wiper (11), the other wiper (12) and their pivot mesh (13: two vertices, `pos` = the spindle, `nrm` = the sweep axis out of the glass). The wipers are black plastic without a texture, drawn with the glass; `wiper_angle(time, rain)` parks them below a drizzle and sweeps them (1.15 rad) otherwise. The raindrops are drawn in `fs_glass` on a lattice in the glass's object space.

The fifteenth mesh is the dash (the instrument cluster, class 16, backlit by darkness, and the four RPM LEDs, class 17, the LED's number in the green channel); the game appends it to the interior. The LEDs show `dash_rpm(speed, max_speed)` (the engine-sound curve, there is no gearbox) in the uniform's `rt.w`. The driver's gloves are skinned in the source model and collapse without the rig, so the hands are not drawn separately.

`rally_car_normal.jpg` (a ninth argument to `carconv`, after the three budgets) is the normal-map atlas: the livery's layout at half size, each rectangle holding the normal map of the material that uses that texture, flat (128, 128, 255) elsewhere. `fs_model` tilts the normal of textured paint, plastic, chrome and rubber by it (`map_normal`: a cotangent frame from screen-space derivatives, so the mesh needs no tangents) within about 14 to 40 m of the camera. The ground gets screen-space bump mapping (`bump`) from the terrain texture's brightness and, on the road, from fine noise.

The rally car's paint (class 0 with a texture) is shaded by `shade_coat`: the base (satin, roughness 0.30, bump-mapped) is dimmed by the coat's Fresnel reflection and a second GGX lobe (F0 0.04, roughness 0.07) on the unperturbed surface normal adds the sun's glint and the sky's reflection.

Simplification notes (`tools/carconv`): the body is simplified with `LockBorder` and a small error (0.03), the cabin and the steering wheel with `LockBorder` and an error of 0.004 (the driver 0.02): a looser error bridged the windscreen opening with big triangles (a black wedge across the windscreen in the cockpit view and a dark triangle on the glass from outside), so the cabin keeps about 25 000 triangles, the steering wheel 3 400, the driver 4 800. When the camera is within 1.5 m of the car its body is drawn with the back faces culled (`pl_model_back`).

Файл может нести четырнадцать мешей: после водителя (10) идут дворник водителя (11), второй дворник (12) и их меш осей (13: две вершины, `pos` — ось вращения, `nrm` — ось взмаха наружу от стекла). Дворники — чёрный пластик без текстуры, рисуются вместе со стёклами; `wiper_angle(time, rain)` паркует их в морось и иначе ведёт взмах (1,15 рад). Капли дождя рисуются в `fs_glass` на решётке в объектном пространстве стекла.

Пятнадцатый меш — панель (комбинация приборов, класс 16, подсвечивается темнотой, и четыре светодиода оборотов, класс 17, номер светодиода в зелёном канале); игра дописывает его к салону. Светодиоды показывают `dash_rpm(speed, max_speed)` (кривая звука двигателя, коробки передач нет) через `rt.w` униформа. Перчатки водителя в исходной модели скиннятся и без скелета схлопываются, поэтому руки отдельно не рисуются.

`rally_car_normal.jpg` (десятый аргумент `carconv`, после трёх бюджетов) — атлас карт нормалей: раскладка ливреи в половинном размере, в каждом прямоугольнике карта нормалей материала, который использует эту текстуру, в остальном плоско (128, 128, 255). `fs_model` наклоняет нормаль окрашенного кузова, пластика, хрома и резины по ней (`map_normal`: котангенс-базис из экранных производных, тангенты в меше не нужны) в пределах примерно 14–40 м от камеры. Земля получает экранный бамп-маппинг (`bump`) по яркости текстуры местности, дорога — по мелкому шуму.

Краска раллийной машины (класс 0 с текстурой) считается в `shade_coat`: основа (атлас, шероховатость 0,30, с бампом) затемняется френелевским отражением лака, а второй лепесток GGX (F0 0,04, шероховатость 0,07) на гладкой нормали поверхности (без бампа) добавляет блик солнца и отражение неба.

Примечания по упрощению (`tools/carconv`): кузов упрощается с `LockBorder` и малой ошибкой (0,03), салон и руль — с `LockBorder` и ошибкой 0,004 (водитель 0,02): более слабая ошибка перекрывала проём лобового стекла большими треугольниками (чёрный клин поперёк лобового стекла на виде из салона и тёмный треугольник на стекле снаружи), поэтому в салоне около 25 000 треугольников, в руле 3 400, у водителя 4 800. Когда камера ближе 1,5 м к машине, её кузов рисуется с отсечением задних граней (`pl_model_back`).


## Tyre sets and grip

`rust/surfaces.json` -> `tyre_sets`: for each wheel set (`tarmac`, `gravel`, `snow`) and surface two multipliers `[grip, lateral]` (0.5 to 1.5; a missing object, set or surface is 1.0, an unknown name or a number out of range refuses the file). `grip::tyre_factor(set, surface)` gives them; `Vehicle::step` multiplies the car's `mu` by the first and its side stiffness by the second, next to the surface's own grip. Defaults: the tarmac set is the baseline the cars were tuned with (1.0 everywhere); gravel tyres give 1.04-1.08 on gravel, dirt, grass and sand and 0.90 on asphalt; snow tyres 0.85 on asphalt and 0.95-1.0 elsewhere. A tuning file next to the profile overrides it like the rest of the table.

## Комплекты шин и сцепление

`rust/surfaces.json` -> `tyre_sets`: для каждого комплекта колёс (`tarmac`, `gravel`, `snow`) и покрытия два множителя `[сцепление, боковое]` (от 0,5 до 1,5; отсутствующий объект, комплект или покрытие — 1,0, неизвестное имя или число вне диапазона отвергает файл). `grip::tyre_factor(комплект, покрытие)` возвращает их; `Vehicle::step` умножает `mu` машины на первое и боковую жёсткость на второе рядом с собственным сцеплением покрытия. По умолчанию: асфальтовый комплект — базовый, под него настроены машины (везде 1,0); гравийные шины дают 1,04–1,08 на гравии, грунте, траве и песке и 0,90 на асфальте; зимние — 0,85 на асфальте и 0,95–1,0 на остальном. Файл настроек рядом с профилем переопределяет таблицу, как и остальное.

## Wipers and raindrops on the glass

The moving wiper is the model's arm (`int_wiper_*_base`, which ends in the round spindle cap) and blade (`int_wiper_*`) together (meshes 11 and 12); the spindle is the arm's end farthest from the blade (it matches the model's own `WIPER_L`/`WIPER_R` dummies), the sweep axis is the one of the model's rig bones (`WIPER_L1`, `WIPER_R1`), the spindle's or the windscreen's normal that the blade lies flattest against. `wiper_cycle`/`wiper_angle_at` give the car-wiper timing (intermittent in a drizzle, slow, fast) and the sin-squared sweep.

`raster/glassdrops.rs` holds the physics of the water on the glass (the constants are tuned for the game; the model follows the cited papers, the numbers are not copied from them):
* A drop on glass is pinned by contact-angle hysteresis; it slides when the force along the glass exceeds Furmidge's `gamma (cos theta_r - cos theta_a) D` ([Motion of rain drops on a car side window](https://emergent-scientist.edp-open.org/articles/emsci/full_html/2019/01/emsci180004/emsci180004.html); the same depinning under wind and gravity: [Wind- and Gravity-Forced Drop Depinning](https://arxiv.org/pdf/2009.04059)).
* Large drops are mass points moved by hysteresis, gravity and air drag, small ones are static ([Animation of Water Droplets on a Hydrophobic Windshield](https://www.researchgate.net/publication/268438621_Animation_of_Water_Droplets_on_a_Hydrophobic_Windshield)): here up to 32 runners are simulated on the windscreen's plane (`Windscreen::from_glass` finds it in the glass mesh) and the pinned drops are a lattice evaluated in `fs_glass`.
* The air pushes with `1/2 rho u^2 Cd A` (`u` the air near the glass, half the car's speed) and gravity pulls with `rho g V sin(alpha)`; the pinning grows with the drop's width, so there is a critical radius, about 2.5 mm on the windscreen at rest and lower as the car goes faster (the air starts to move drops of that size from about 65 km/h).
* Evaporation is diffusion-limited: `d(r^2)/dt` is constant and proportional to `1 - RH` ([Evaporation of Sessile Droplets, Wilson et al. 2023](https://strathprints.strath.ac.uk/82256/7/Wilson_etal_ARFM_2023_Evaporation_of_sessile_droplets.pdf)); the air at the glass starts saturated and dries towards 55 % in a minute, moving air and a sunny pane speed it up, and the game runs it 60 times faster than life.

## Дворники и капли на стёклах

Двигается дворник модели целиком: рычаг (`int_wiper_*_base`, заканчивающийся круглой головкой оси) и щётка (`int_wiper_*`) — меши 11 и 12; ось — конец рычага, самый дальний от щётки (она совпадает с родными пустышками модели `WIPER_L`/`WIPER_R`), ось качания — та из осей арматуры модели (`WIPER_L1`, `WIPER_R1`), оси шпинделя или нормали лобового стекла, к которой щётка прилегает плотнее всего. `wiper_cycle`/`wiper_angle_at` задают ход как у автомобильных дворников (прерывистый в морось, медленный, быстрый) и качание по sin².

В `raster/glassdrops.rs` — физика воды на стекле (константы подобраны для игры; модель следует перечисленным работам, числа из них не скопированы):
* Капля на стекле удерживается гистерезисом краевого угла; она скользит, когда сила вдоль стекла превышает `gamma (cos theta_r - cos theta_a) D` по Фурмиджу ([Motion of rain drops on a car side window](https://emergent-scientist.edp-open.org/articles/emsci/full_html/2019/01/emsci180004/emsci180004.html); то же освобождение капли под ветром и тяжестью: [Wind- and Gravity-Forced Drop Depinning](https://arxiv.org/pdf/2009.04059)).
* Крупные капли — материальные точки, движимые гистерезисом, тяжестью и сопротивлением воздуха, мелкие неподвижны ([Animation of Water Droplets on a Hydrophobic Windshield](https://www.researchgate.net/publication/268438621_Animation_of_Water_Droplets_on_a_Hydrophobic_Windshield)): здесь до 32 бегущих считаются в плоскости лобового стекла (`Windscreen::from_glass` находит её в меше стёкол), а неподвижные капли — решётка, вычисляемая в `fs_glass`.
* Воздух давит силой `1/2 rho u^2 Cd A` (`u` — воздух у стекла, половина скорости машины), тяжесть тянет `rho g V sin(alpha)`; удержание растёт с шириной капли, поэтому есть критический радиус: около 2,5 мм на лобовом на месте и меньше на ходу (воздух начинает двигать капли такого размера примерно с 65 км/ч).
* Испарение ограничено диффузией: `d(r^2)/dt` постоянна и пропорциональна `1 - RH` ([Evaporation of Sessile Droplets, Wilson et al. 2023](https://strathprints.strath.ac.uk/82256/7/Wilson_etal_ARFM_2023_Evaporation_of_sessile_droplets.pdf)); воздух у стекла сначала насыщен и за минуту подсыхает к 55 %, движущийся воздух и нагретое солнцем стекло ускоряют, а игра гонит всё это в 60 раз быстрее жизни.
