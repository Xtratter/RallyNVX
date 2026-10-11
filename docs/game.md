# Game

[Русский](#игра) · **English**

Raytrace has a game mode next to the scene viewer: off-road free drive on a random level.

## Controls

| | Android | Desktop (`raytrace-desktop --game`) |
|---|---|---|
| Throttle / reverse | left stick up / down | W S, arrows |
| Steer | left stick left / right | A D, arrows |
| Brake | right stick down | Space |
| Handbrake (drift) | "Handbrake" button (hold) | Left Shift |
| Reset (flipped car) | "Reset" button | R |
| New random level | "New level" button | N |

## Seeds

Every level is generated from a number (seed): the terrain, rocks and trees are the same for the same seed on every device. The seed is shown in the HUD (Android) and the window title (desktop). Seeds are 0..16777215. Replay a level on desktop with `raytrace-desktop --game 42` or `RAYTRACE_SEED=42`.

## Quality

On Android the start screen has Quality: Low (0.5x), Medium (0.7x) and High (1.0x) of the screen resolution.

## Missions

The main menu opens a list of four random missions: two **deliveries** and two **checkpoint races**. Tap a card for the briefing, then Start. The goal is a tall yellow pillar, race checkpoints are rings (the next one is pink); the HUD shows the timer, an arrow with the distance to the target and, for deliveries, the cargo condition.

- **Time limit**: about 1.6 x the path length at 12 m/s plus 20 s. The clock starts with the first throttle.
- **Cargo** (deliveries only, 100%): a hard hit (horizontal speed change above 6 m/s in a step), a hard landing after a jump, a rollover (8% per second) and the Reset button (5%) damage it; at 0% the mission fails.
- **Medals**: Gold = within 60% of the time limit and cargo at least 90%; Silver = within 80% and at least 60%; otherwise Bronze. Races use time only.
- **Reward** = base x 1 / 1.5 / 2.2 for Bronze / Silver / Gold. Replaying a mission pays the difference when you improve the medal, otherwise half.
- **Refresh** rolls a new list. Progress is saved in `files/profile.json` (desktop: `~/.config/raytrace/profile.json`); a damaged file is kept as `profile.json.bak`.
- Desktop: `RAYTRACE_MISSION=0..3 raytrace-desktop --game` starts a mission, `M` abandons it.

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

## City

Every level has one or two cities (about 530 x 530 m) standing on a flat plateau a few hundred metres from the start: a grid of streets, blocks, parks and buildings from 4 m to 90 m, the tallest in the centre. Buildings are solid: the car stops at them, an aircraft that touches a wall or the underside of a building crashes. A helicopter or drone can land softly on a roof (descent up to 3 m/s) and take off again; sling cargo rests on roofs. The Quality setting also sets how far the city is drawn: 400 m (Low), 600 m (Medium), 700 m (High). Desktop: `RAYTRACE_AT_CITY=1 raytrace-desktop --game` starts next to the first city, `RAYTRACE_CITY_RADIUS=500` sets the draw distance.

## Limits of this version

No garage, rescue missions or NPC orders yet, and missions are not city-aware (goals avoid cities), and only the raster renderer (mode C). The ray-traced Scenes mode is unchanged.

---

# Игра

Рядом с просмотром сцен есть игровой режим: свободная езда по случайному уровню на бездорожье.

## Управление

| | Android | Десктоп (`raytrace-desktop --game`) |
|---|---|---|
| Газ / задний ход | левый стик вверх / вниз | W S, стрелки |
| Руль | левый стик влево / вправо | A D, стрелки |
| Тормоз | правый стик вниз | Пробел |
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
- **Обновить** создаёт новый список. Прогресс хранится в `files/profile.json` (десктоп: `~/.config/raytrace/profile.json`); повреждённый файл сохраняется как `profile.json.bak`.
- Десктоп: `RAYTRACE_MISSION=0..3 raytrace-desktop --game` запускает миссию, `M` прерывает.

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

## Город

В каждом уровне есть один или два города (около 530 x 530 м) на ровном плато в нескольких сотнях метров от старта: сетка улиц, кварталы, парки и здания от 4 до 90 м, самые высокие в центре. Здания твёрдые: машина упирается в них, аппарат, коснувшийся стены или низа здания, разбивается. Вертолёт или дрон может мягко сесть на крышу (снижение до 3 м/с) и снова взлететь; груз на подвесе лежит на крышах. Настройка «Качество» задаёт и дальность прорисовки города: 400 м (Низкое), 600 м (Среднее), 700 м (Высокое). Десктоп: `RAYTRACE_AT_CITY=1 raytrace-desktop --game` стартует рядом с первым городом, `RAYTRACE_CITY_RADIUS=500` задаёт дальность прорисовки.

## Ограничения этой версии

Пока нет гаража, спасательных миссий и заказов NPC, а миссии не знают о городах (цели обходят города), и только растровый рендер (режим C). Режим «Сцены» с трассировкой не менялся.
