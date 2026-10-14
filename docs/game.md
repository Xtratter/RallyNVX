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

## Seeds

Every level is generated from a number (seed): the terrain, rocks and trees are the same for the same seed on every device. The seed is shown in the HUD (Android) and the window title (desktop). Seeds are 0..16777215. Replay a level on desktop with `raytrace-desktop --game 42` or `RAYTRACE_SEED=42`.

## Quality

On Android the start screen has Quality: Low (0.5x), Medium (0.7x) and High (1.0x) of the screen resolution.

## Missions

The main menu opens a list of six random missions: ground **deliveries** and **checkpoint races**, air **rings**, **sling** cargo and **recon** (see Flight), some of them urban. Tap a card for the briefing, then Start. The goal is a tall yellow pillar, race checkpoints are rings (the next one is pink); the HUD shows the timer, an arrow with the distance to the target and, for deliveries, the cargo condition.

- **Time limit**: about 1.6 x the path length at 12 m/s plus 20 s. The clock starts with the first throttle.
- **Cargo** (deliveries only, 100%): a hard hit (horizontal speed change above 6 m/s in a step), a hard landing after a jump, a rollover (8% per second) and the Reset button (5%) damage it; at 0% the mission fails.
- **Medals**: Gold = within 60% of the time limit and cargo at least 90%; Silver = within 80% and at least 60%; otherwise Bronze. Races use time only.
- **Reward** = base x 1 / 1.5 / 2.2 for Bronze / Silver / Gold. Replaying a mission pays the difference when you improve the medal, otherwise half.

**Urban missions** (marked "City" on the card) take place in the level's city and start on a street at its west edge. The delivery goes through the streets (buildings are solid), the street race follows the turning intersections, rings hang over a street between the houses, and the sling mission hooks the cargo on one roof and sets it down on the green pad of another roof.

- **Refresh** rolls a new list. Progress is saved in `files/profile.json` (desktop: `~/.config/raytrace/profile.json`); a damaged file is kept as `profile.json.bak`.
- Desktop: `RAYTRACE_MISSION=0..5 raytrace-desktop --game` starts a mission, `M` abandons it.

## Flight

Air missions use a helicopter or a drone (the card shows which). Sticks released = the aircraft hovers and holds its altitude.

| | Android | Desktop |
|---|---|---|
| Turn | left stick left / right | Q / E |
| Climb / descend | left stick up / down | Up / Down |
| Forward / back | right stick up / down | W / S |
| Strafe | right stick left / right | A / D |

- **Landing**: soft (descent up to 3 m/s, tilt up to 20 degrees) is fine; a harder touchdown, a steep tilt at contact or hitting a tree or rock is a crash and the mission fails.
- **Rings**: fly through every ring in order (helicopter ring radius 8 m, drone 6 m).
- **Sling** (helicopter): hover lower than 4 m above the cargo for 1.5 s to hook it, carry it to the green pad and set it down gently inside the circle; ground impacts and wild swinging damage the cargo.
- **Recon** (drone): hover for 2 s inside every zone (40 m), in any order.
- Medals, rewards and the profile work as for ground missions. The drone is faster and tilts more; the helicopter is steadier.

## Render modes

Settings has **Render mode**: *Raster* (the fast classic look) or *Hybrid*. Hybrid adds soft sun shadows (sharp at the foot of a tree, softer further away), long shadows of hills from a height map (dawn and dusk shadows reach far across the land) and ambient occlusion (hollows, tree and rock bases and building feet are darker). Hybrid is on by default only with High graphics quality and costs roughly 15-20 % of the frame rate (about 80 fps against 100 in a forest on a POCO F3). **Auto fallback to Raster** (off by default) drops to Raster for the rest of the drive when the frame rate stays below 50 for four seconds, and the HUD says so.

## Settings

The main menu and the pause panel open **Settings**: graphics quality (applies at the next start), steering (buttons, wheel or tilt), pedal force (full on touch or adjustable), sensitivity (low / normal / high), volume (0 / 25 / 50 / 75 / 100 %), the side of the pedals (the wheel goes to the other side), recalibrating the tilt centre (the pose at the start of a drive is the centre) and showing the hints again. The helicopter and the drone keep their two sticks. A short hint appears the first time you drive with each steering type.

## Rescue

The sixth mission card rotates with each Refresh: sling, recon, **rescue**. In a rescue you fly the helicopter to 1-3 survivor sites (more on higher difficulty; 1-3 people each, at most 4 aboard), land inside the green circle and wait 3 seconds while they board, then land on the base pad and wait 2 seconds to unload. People waiting at a site are shown as orange-vested figures. A touchdown faster than 1.2 m/s hurts the passengers (integrity falls by 12 per m/s over 1.2; 3 m/s is already a crash); at 0 the mission fails. The medal uses time and integrity like the sling cargo; the reward adds 50 per person. The HUD shows sites done, people aboard and the boarding progress.

## Sound

All sound is synthesised in code (mono, 44.1 kHz, no files). The car has an engine whose pitch follows the speed and throttle, rolling noise on the ground and in water and tyre squeal in slides; the helicopter has blade thumps and a rotor hum, the drone a high buzz; wind grows with speed and height. One-shot sounds: touchdown, crash, splash, checkpoint chime, boarding tick, mission success and failure tunes. The 🔊/🔇 button at the top left (under the pause button) switches sound off and on and remembers the choice; the sound fades out while the game is paused or the screen is left. There is no music and no desktop sound yet; a volume slider comes with the settings screen.

## City

Every level has one or two cities (about 530 x 530 m) standing on a flat plateau a few hundred metres from the start: a grid of streets, blocks, parks and buildings from 4 m to 90 m, the tallest in the centre. Buildings are solid: the car stops at them, an aircraft that touches a wall or the underside of a building crashes. A helicopter or drone can land softly on a roof (descent up to 3 m/s) and take off again; sling cargo rests on roofs. The Quality setting also sets how far the city is drawn: 300 m (Low), 400 m (Medium), 450 m (High). Desktop: `RAYTRACE_AT_CITY=1 raytrace-desktop --game` starts next to the first city, `RAYTRACE_CITY_RADIUS=500` sets the draw distance.

## Graphics

The terrain is textured with generated materials (grass, dirt, sand, rock) blended by height, slope and noise. Vehicles are detailed generated models (paint, glass, chrome, lamps, spinning wheels, rotors and propellers) and the car leaves tyre tracks. Trees (conifers and broadleaf), bushes, rocks and grass tufts are generated models (grass is drawn within 40 m: none on Low, 1500 tufts on Medium, 3000 on High). The sun moves while you play (a full cycle takes 12 minutes, dawn - noon - dusk, no night) and the light, sky and fog colours follow it; the sky has drifting clouds. Low ground fills with shallow lakes: the car can drive through them with extra drag and splashes, aircraft are unaffected, and mission points are never in water. Cities have ten facade colours, roof cornices, street lamps and even asphalt with lane marks. Nothing is loaded from files: all textures and meshes are made in code.

## Limits of this version

No garage, rescue missions or NPC orders yet, and only the raster renderer (mode C). The ray-traced Scenes mode is unchanged.

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

## Seed

Каждый уровень строится из числа (seed): ландшафт, камни и деревья на любом устройстве одинаковы для одного seed. Seed виден в HUD (Android) и в заголовке окна (десктоп). Seed: 0..16777215. Повторить уровень на десктопе: `raytrace-desktop --game 42` или `RAYTRACE_SEED=42`.

## Качество

На Android на стартовом экране есть «Качество»: Низкое (0,5x), Среднее (0,7x), Высокое (1,0x) от разрешения экрана.

## Миссии

Главное меню открывает список из четырёх случайных миссий: две **доставки** и две **гонки по чекпойнтам**. Нажмите на карточку, чтобы увидеть брифинг, затем «Старт». Цель — высокий жёлтый столб, чекпойнты гонки — кольца (следующий розовый); в HUD таймер, стрелка с расстоянием до цели и, для доставок, состояние груза.

- **Лимит времени**: примерно 1,6 x длина пути при 12 м/с плюс 20 с. Время идёт с первого нажатия газа.
- **Груз** (только доставки, 100%): портят сильный удар (изменение горизонтальной скорости больше 6 м/с за шаг), жёсткая посадка после прыжка, переворот (8% в секунду) и кнопка «Сброс» (5%); при 0% миссия провалена.
- **Медали**: золото — уложились в 60% лимита и груз не менее 90%; серебро — в 80% и не менее 60%; иначе бронза. В гонках только время.
- **Награда** = база x 1 / 1,5 / 2,2 за бронзу / серебро / золото. Повторное прохождение платит разницу при улучшении медали, иначе половину.

**Городские миссии** (на карточке пометка «Город») проходят в городе уровня и стартуют на улице у его западного края. Доставка идёт по улицам (здания твёрдые), гонка проходит поворотные перекрёстки, кольца висят над улицей между домами, а в миссии с подвесом груз цепляют на одной крыше и опускают на зелёную площадку другой.

- **Обновить** создаёт новый список. Прогресс хранится в `files/profile.json` (десктоп: `~/.config/raytrace/profile.json`); повреждённый файл сохраняется как `profile.json.bak`.
- Десктоп: `RAYTRACE_MISSION=0..5 raytrace-desktop --game` запускает миссию, `M` прерывает.

## Полёт

В воздушных миссиях вы летите на вертолёте или дроне (на карточке указано, на каком). Стики отпущены — аппарат висит и держит высоту.

| | Android | Десктоп |
|---|---|---|
| Поворот | левый стик влево / вправо | Q / E |
| Набор / снижение | левый стик вверх / вниз | Up / Down |
| Вперёд / назад | правый стик вверх / вниз | W / S |
| Сдвиг вбок | правый стик влево / вправо | A / D |

- **Посадка**: мягкая (снижение до 3 м/с, наклон до 20 градусов) — нормально; жёстче, крутой наклон при касании или удар о дерево или камень — авария, миссия провалена.
- **Кольца**: пролетите все кольца по порядку (радиус кольца у вертолёта 8 м, у дрона 6 м).
- **Подвес** (вертолёт): повисите ниже 4 м над грузом 1,5 с, чтобы зацепить, довезите до зелёной площадки и мягко опустите внутри круга; удары о землю и сильное раскачивание портят груз.
- **Разведка** (дрон): повисите 2 с в каждой зоне (40 м), в любом порядке.
- Медали, награды и профиль работают как в наземных миссиях. Дрон быстрее и сильнее наклоняется; вертолёт устойчивее.

## Режимы рендера

В настройках есть **Режим рендера**: *Растр* (быстрый классический вид) или *Гибрид*. Гибрид добавляет мягкие тени от солнца (резкие у основания дерева, дальше мягче), длинные тени холмов по карте высот (тени на рассвете и закате далеко ложатся по земле) и затенение углов (впадины, основания деревьев и камней и подножия зданий темнее). Гибрид включён по умолчанию только при высоком качестве графики и стоит примерно 15–20 % частоты кадров (около 80 fps против 100 в лесу на POCO F3). **Откат на Растр** (по умолчанию выключен) переключает на Растр до конца поездки, если частота кадров держится ниже 50 в течение четырёх секунд, и HUD сообщает об этом.

## Настройки

Главное меню и панель паузы открывают **Настройки**: качество графики (применится при следующем запуске), руль (кнопки, колесо или наклон), усилие педалей (полное по нажатию или регулируемое), чувствительность (низкая / обычная / высокая), громкость (0 / 25 / 50 / 75 / 100 %), сторона педалей (руль уходит на другую), перекалибровка центра наклона (центр — поза в начале поездки) и повторный показ подсказок. Вертолёт и дрон по-прежнему управляются двумя стиками. При первой поездке с каждым видом руля появляется короткая подсказка.

## Спасение

Шестая карточка миссий меняется при каждом обновлении: подвес, разведка, **спасение**. В спасении вертолёт летит к 1–3 площадкам со спасёнными (на высокой сложности больше; по 1–3 человека, на борту не больше 4), садится в зелёный круг и ждёт 3 секунды, пока люди садятся, затем садится на площадку базы и ждёт 2 секунды высадки. Ждущие люди — фигурки в оранжевых жилетах. Касание быстрее 1,2 м/с ранит пассажиров (целостность падает на 12 за каждый м/с сверх 1,2; 3 м/с — уже авария); при нуле миссия проваливается. Медаль по времени и целостности, как у груза на подвесе; награда плюс 50 за человека. В HUD видно пройденные площадки, людей на борту и прогресс посадки.

## Звук

Весь звук синтезируется кодом (моно, 44,1 кГц, без файлов). У машины двигатель, тон которого следует за скоростью и газом, шум качения по земле и по воде и визг шин в заносе; у вертолёта хлопки лопастей и гул ротора, у дрона высокое жужжание; ветер растёт со скоростью и высотой. Разовые звуки: касание, авария, брызги, сигнал чекпойнта, щелчок посадки людей, мелодии успеха и провала миссии. Кнопка 🔊/🔇 слева сверху (под кнопкой паузы) включает и выключает звук и запоминает выбор; на паузе и при выходе с экрана звук затухает. Музыки и звука на десктопе пока нет; регулятор громкости появится с экраном настроек.

## Город

В каждом уровне есть один или два города (около 530 x 530 м) на ровном плато в нескольких сотнях метров от старта: сетка улиц, кварталы, парки и здания от 4 до 90 м, самые высокие в центре. Здания твёрдые: машина упирается в них, аппарат, коснувшийся стены или низа здания, разбивается. Вертолёт или дрон может мягко сесть на крышу (снижение до 3 м/с) и снова взлететь; груз на подвесе лежит на крышах. Настройка «Качество» задаёт и дальность прорисовки города: 300 м (Низкое), 400 м (Среднее), 450 м (Высокое). Десктоп: `RAYTRACE_AT_CITY=1 raytrace-desktop --game` стартует рядом с первым городом, `RAYTRACE_CITY_RADIUS=500` задаёт дальность прорисовки.

## Графика

Рельеф покрыт сгенерированными материалами (трава, земля, песок, скала), которые смешиваются по высоте, наклону и шуму. Техника — детальные сгенерированные модели (краска, стекло, хром, фары, вращающиеся колёса, винты), а машина оставляет следы шин. Деревья (хвойные и лиственные), кусты, камни и пучки травы — сгенерированные модели (трава рисуется в 40 м: на Низком её нет, на Среднем 1500 пучков, на Высоком 3000). Солнце движется во время игры (полный цикл 12 минут, рассвет — полдень — закат, без ночи), цвета света, неба и тумана следуют за ним; в небе плывут облака. В низинах появляются мелкие озёра: машина проезжает их с дополнительным сопротивлением и брызгами, аппараты их не замечают, а точки миссий никогда не стоят в воде. В городах десять цветов фасадов, карнизы на крышах, уличные фонари и ровный асфальт с разметкой. Ничего не грузится из файлов: все текстуры и меши строятся в коде.

## Ограничения этой версии

Пока нет гаража, спасательных миссий и заказов NPC, и только растровый рендер (режим C). Режим «Сцены» с трассировкой не менялся.
