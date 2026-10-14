# Game

[Русский](#игра) · **English**

Raytrace has a game mode next to the scene viewer: off-road free drive on a random level.

## Controls

| | Android | Desktop (`raytrace-desktop --game`) |
|---|---|---|
| Throttle | gas pedal (right, taller): full gas on touch; with "Adjustable" pedal force in Settings the lower you press, the more gas | W, arrow up |
| Brake / reverse | brake pedal (left of the gas): full brake while moving forward, reverse when stopped or rolling backwards | S, arrow down |
| Steer | two arrow buttons (default), an on-screen wheel (drag it around its centre) or tilt of the phone (Settings) | A D, arrows |
| Handbrake (drift) | "Handbrake" button (hold) | Left Shift |
| Reset (flipped car) | "Reset" button | R |
| New random level | "New level" button | N |
| Look around | drag a finger on free space (also while driving) | - |

The chase camera: drag a finger on free space to swing it round the car (left/right) and to raise or lower it (up/down: lowered to the horizon it shows the sky, the sun and the moon). When you drive forward faster than about 7 km/h and the finger is up, the camera swings back behind the car within half a second. When you reverse (hold reverse for a third of a second, or roll backwards) the camera swings round in front of the car and looks back, so you see what is behind; driving forward swings it back.

## Seeds

Every level is generated from a number (seed): the terrain, rocks and trees are the same for the same seed on every device. The seed is shown in the HUD (Android) and the window title (desktop). Seeds are 0..16777215. Replay a level on desktop with `raytrace-desktop --game 42` or `RAYTRACE_SEED=42`.

## Quality

On Android the start screen has Quality: Low (0.5x), Medium (0.7x) and High (1.0x) of the screen resolution.

## Missions

The delivery and race missions were removed in 2.14.0: the game is a free drive for now, and a rally mode (special stages with times) is planned. The profile (`files/profile.json`, desktop: none) keeps the money and the garage; a damaged file is kept as `profile.json.bak`.

## Garage

The main menu has a **Garage**: a turntable view of the car, arrows to switch between cars and a panel with the money, the **Buy / Select** button and four upgrades per car: Engine (+8 / +16 / +25 % thrust, a little more top speed), Tyres (+5 / +10 / +15 % grip), Suspension (+10 / +20 / +30 % stiffness and damping) and Brakes (+20 / +40 / +60 %). A level costs 1x, 2x and 4x the car's upgrade price (300 for the off-roader, 750 for the Vulpes), but everything in the garage is free for now: nothing earns money until the rally mode, so the garage shows "Free" and purchases leave the balance alone. Two cars: the off-roader (yours from the start) and the **Vulpes WRC** (3000, free for now): a light rally hatchback (a simplified model based on "Peugeot 206 WRC" by vecarz, CC BY-NC-SA 4.0, see `rust/assets/ASSET-LICENSE.md`), quicker and grippier but stiffer over bumps. The car you select, with its upgrades, is used in free drive and in every mission. Cars and prices live in `vehicles.json` (see docs/vehicles.md).

## Render modes

Settings has **Render mode**: *Raster* (the fast classic look) or *Hybrid*. Hybrid adds soft sun shadows (sharp at the foot of a tree, softer further away), long shadows of hills from a height map (dawn and dusk shadows reach far across the land) and ambient occlusion (hollows, tree and rock bases and building feet are darker). Hybrid is on by default only with High graphics quality and costs roughly 15-20 % of the frame rate (about 80 fps against 100 in a forest on a POCO F3). **Auto fallback to Raster** (off by default) drops to Raster for the rest of the drive when the frame rate stays below 50 for four seconds, and the HUD says so; it comes back when you restart the game or pick Hybrid again. Hybrid shadows are slightly softer than Raster ones everywhere, and the first six seconds of a drive never count towards the fallback.

## Settings

The main menu and the pause panel open **Settings**: graphics quality (applies at the next start), steering (buttons, wheel or tilt), pedal force (full on touch or adjustable), sensitivity (low / normal / high), volume (0 / 25 / 50 / 75 / 100 %), the side of the pedals (the wheel goes to the other side), recalibrating the tilt centre (the pose at the start of a drive is the centre) and showing the hints again. A short hint appears the first time you drive with each steering type.

## Sound

All sound is synthesised in code (mono, 44.1 kHz, no files). The car has an engine whose pitch follows the speed and throttle, rolling noise on the ground and in water and tyre squeal in slides; wind grows with speed. One-shot sounds: touchdown, crash, splash, checkpoint chime, mission success and failure tunes. The 🔊/🔇 button at the top left (under the pause button) switches sound off and on and remembers the choice; the sound fades out while the game is paused or the screen is left. There is no music and no desktop sound yet; a volume slider comes with the settings screen.

## City

Every level has one or two cities (about 530 x 530 m) standing on a flat plateau a few hundred metres from the start: a grid of streets, blocks, parks and buildings from 4 m to 90 m, the tallest in the centre. Buildings are solid: the car stops at them. The Quality setting also sets how far the city is drawn: 300 m (Low), 400 m (Medium), 450 m (High). Desktop: `RAYTRACE_AT_CITY=1 raytrace-desktop --game` starts next to the first city, `RAYTRACE_CITY_RADIUS=500` sets the draw distance.

## Graphics

The terrain is textured with generated materials (grass, dirt, sand, rock) blended by height, slope and noise. Vehicles are detailed generated models (paint, glass, chrome, lamps, spinning wheels) and the car leaves tyre tracks. Trees (conifers and broadleaf), bushes, rocks and grass tufts are generated models (grass is drawn within 40 m: none on Low, 1500 tufts on Medium, 3000 on High). The sun and the moon move while you play (a full day takes 12 minutes: sunrise, noon, sunset and a dark moonlit night with stars; a level always starts by day) and the light, sky and fog colours follow them; the sky has drifting clouds, a sun disc, a moon with phases and stars (the chase camera sees the sky up to about 17 degrees above the horizon, so the sun and the moon show when they are low); at night windows and street lamps glow. At dusk and at night the car switches its headlights on by itself: a light cone lights the ground and objects ahead, tail lights glow red (brighter when braking) and white reverse lights glow when the car backs up. Low ground fills with shallow lakes: the car can drive through them with extra drag and splashes, and mission points are never in water. Cities have ten facade colours, roof cornices, street lamps and even asphalt with lane marks. Nothing is loaded from files: all textures and meshes are made in code.

## Limits of this version

No rally mode and no NPC orders yet, and only the raster and hybrid renderers. The ray-traced Scenes mode is unchanged.

---

# Игра

Рядом с просмотром сцен есть игровой режим: свободная езда по случайному уровню на бездорожье.

## Управление

| | Android | Десктоп (`raytrace-desktop --game`) |
|---|---|---|
| Газ | педаль газа (справа, выше): полный газ по нажатию; при «Регулируемом» усилии в настройках чем ниже нажать, тем больше газа | W, стрелка вверх |
| Тормоз / задний ход | педаль тормоза (слева от газа): полный тормоз при движении вперёд, задний ход когда стоишь или катишься назад | S, стрелка вниз |
| Руль | две кнопки-стрелки (по умолчанию), экранный руль (крутить пальцем вокруг центра) или наклон телефона (Настройки) | A D, стрелки |
| Ручник (занос) | кнопка «Ручник» (удерживать) | Левый Shift |
| Сброс (машина перевернулась) | кнопка «Сброс» | R |
| Новый случайный уровень | кнопка «Новый уровень» | N |
| Осмотреться | провести пальцем по свободному месту (и на ходу) | - |

Камера погони: проведите пальцем по свободному месту, чтобы повернуть её вокруг машины (влево и вправо) и поднять или опустить (вверх и вниз: опущенная к горизонту, она показывает небо, солнце и луну). Когда вы едете вперёд быстрее примерно 7 км/ч и палец поднят, камера за полсекунды возвращается за машину. При движении назад (газ назад дольше трети секунды или машина катится назад) камера уходит вперёд машины и смотрит назад, чтобы было видно, что сзади; при движении вперёд она возвращается.

## Seed

Каждый уровень строится из числа (seed): ландшафт, камни и деревья на любом устройстве одинаковы для одного seed. Seed виден в HUD (Android) и в заголовке окна (десктоп). Seed: 0..16777215. Повторить уровень на десктопе: `raytrace-desktop --game 42` или `RAYTRACE_SEED=42`.

## Качество

На Android на стартовом экране есть «Качество»: Низкое (0,5x), Среднее (0,7x), Высокое (1,0x) от разрешения экрана.

## Миссии

Миссии «доставка» и «гонка» убраны в 2.14.0: пока игра — это свободная езда, а ралли-режим (спецучастки с временем) в планах. Профиль (`files/profile.json`) хранит деньги и гараж; повреждённый файл сохраняется как `profile.json.bak`.

## Гараж

В главном меню есть **Гараж**: вид машины на поворотном круге, стрелки выбора машины и панель с деньгами, кнопкой **Купить / Выбрать** и четырьмя улучшениями для каждой машины: двигатель (+8 / +16 / +25 % тяги, чуть больше максимальной скорости), шины (+5 / +10 / +15 % сцепления), подвеска (+10 / +20 / +30 % жёсткости и демпфирования) и тормоза (+20 / +40 / +60 %). Уровень стоит 1x, 2x и 4x от цены улучшения машины (300 у внедорожника, 750 у Vulpes), но в гараже всё пока бесплатно: деньги зарабатывать нечем до ралли-режима, поэтому гараж показывает «Бесплатно», а покупки не трогают баланс. Две машины: внедорожник (твой с самого начала) и **Vulpes WRC** (3000, пока бесплатно): лёгкий раллийный хэтчбек (упрощённая модель по «Peugeot 206 WRC» от vecarz, CC BY-NC-SA 4.0, см. `rust/assets/ASSET-LICENSE.md`), быстрее и цепче, но жёстче на кочках. Выбранная машина с её улучшениями используется в свободной езде и во всех миссиях. Машины и цены лежат в `vehicles.json` (см. docs/vehicles.md).

## Режимы рендера

В настройках есть **Режим рендера**: *Растр* (быстрый классический вид) или *Гибрид*. Гибрид добавляет мягкие тени от солнца (резкие у основания дерева, дальше мягче), длинные тени холмов по карте высот (тени на рассвете и закате далеко ложатся по земле) и затенение углов (впадины, основания деревьев и камней и подножия зданий темнее). Гибрид включён по умолчанию только при высоком качестве графики и стоит примерно 15–20 % частоты кадров (около 80 fps против 100 в лесу на POCO F3). **Откат на Растр** (по умолчанию выключен) переключает на Растр до конца поездки, если частота кадров держится ниже 50 в течение четырёх секунд, и HUD сообщает об этом; Гибрид вернётся при перезапуске игры или если выбрать его снова. Тени гибрида везде чуть мягче, чем в Растре, а первые шесть секунд поездки в откат не засчитываются.

## Настройки

Главное меню и панель паузы открывают **Настройки**: качество графики (применится при следующем запуске), руль (кнопки, колесо или наклон), усилие педалей (полное по нажатию или регулируемое), чувствительность (низкая / обычная / высокая), громкость (0 / 25 / 50 / 75 / 100 %), сторона педалей (руль уходит на другую), перекалибровка центра наклона (центр — поза в начале поездки) и повторный показ подсказок. При первой поездке с каждым видом руля появляется короткая подсказка.

## Звук

Весь звук синтезируется кодом (моно, 44,1 кГц, без файлов). У машины двигатель, тон которого следует за скоростью и газом, шум качения по земле и по воде и визг шин в заносе; ветер растёт со скоростью. Разовые звуки: касание, авария, брызги, сигнал чекпойнта, мелодии успеха и провала миссии. Кнопка 🔊/🔇 слева сверху (под кнопкой паузы) включает и выключает звук и запоминает выбор; на паузе и при выходе с экрана звук затухает. Музыки и звука на десктопе пока нет; регулятор громкости появится с экраном настроек.

## Город

В каждом уровне есть один или два города (около 530 x 530 м) на ровном плато в нескольких сотнях метров от старта: сетка улиц, кварталы, парки и здания от 4 до 90 м, самые высокие в центре. Здания твёрдые: машина упирается в них. Настройка «Качество» задаёт и дальность прорисовки города: 300 м (Низкое), 400 м (Среднее), 450 м (Высокое). Десктоп: `RAYTRACE_AT_CITY=1 raytrace-desktop --game` стартует рядом с первым городом, `RAYTRACE_CITY_RADIUS=500` задаёт дальность прорисовки.

## Графика

Рельеф покрыт сгенерированными материалами (трава, земля, песок, скала), которые смешиваются по высоте, наклону и шуму. Техника — детальные сгенерированные модели (краска, стекло, хром, фары, вращающиеся колёса), а машина оставляет следы шин. Деревья (хвойные и лиственные), кусты, камни и пучки травы — сгенерированные модели (трава рисуется в 40 м: на Низком её нет, на Среднем 1500 пучков, на Высоком 3000). Солнце и луна движутся во время игры (сутки занимают 12 минут: рассвет, полдень, закат и тёмная ночь при луне со звёздами; уровень всегда начинается днём), цвета света, неба и тумана следуют за ними; в небе плывут облака, видны диск солнца, луна с фазами и звёзды (камера погони видит небо примерно до 17 градусов над горизонтом, поэтому солнце и луна заметны, когда стоят низко); ночью светятся окна и уличные фонари. В сумерки и ночью машина сама включает фары: конус света освещает землю и предметы впереди, задние огни светятся красным (ярче при торможении), а при движении назад горят белые фонари заднего хода. В низинах появляются мелкие озёра: машина проезжает их с дополнительным сопротивлением и брызгами, а точки миссий никогда не стоят в воде. В городах десять цветов фасадов, карнизы на крышах, уличные фонари и ровный асфальт с разметкой. Ничего не грузится из файлов: все текстуры и меши строятся в коде.

## Ограничения этой версии

Пока нет гаража и заказов NPC, и только растровый и гибридный рендер. Режим «Сцены» с трассировкой не менялся.
