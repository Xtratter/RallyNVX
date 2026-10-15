# Rally car model (`rally_car.rcar`, shown in the game as the "Vulpes WRC")

The car mesh is a simplified, vertex-coloured derivative of the Sketchfab model
**"Peugeot 206 WRC | www.vecarz.com"** by **vecarz** (https://sketchfab.com/heynic),
https://sketchfab.com/3d-models/peugeot-206-wrc-wwwvecarzcom-bee2f4e7aecc4e5ea24769af9c20f3e2

License: **Creative Commons Attribution-NonCommercial-ShareAlike 4.0** (https://creativecommons.org/licenses/by-nc-sa/4.0/).

Changes: the brand badges were left out, only the exterior shell, lights, mirrors, spoiler and one wheel were kept, the interior, driver, engine and
underbody were removed, the geometry was decimated (about 12 000 triangles for the body) and the body's textures were scaled down and packed into one 2048 x 2048
livery atlas (`rally_car_livery.jpg`, with UV coordinates in the mesh; small parts keep baked vertex colours); one wheel (tyre, rim), the gravel and snow wheel sets, the brake disc, the brake caliper and the mud flaps are also derived from the same model by `tools/carconv`. This file and `rally_car_livery.jpg`, and only these files, are licensed under CC BY-NC-SA 4.0 (credit the author,
no commercial use, share modifications alike); the rest of the project stays under its own licence (Apache-2.0).
If the game is ever distributed commercially the asset must be replaced by the generated model (`raster/rally_model.rs`).
