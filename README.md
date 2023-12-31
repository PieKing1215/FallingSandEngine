# FallingSandEngine (Rust rewrite)

An experimental falling sand engine written in Rust, successor to [the C++ version](https://github.com/PieKing1215/FallingSandSurvival)

_Note: development of this project is on hiatus._<br>
It's a bit messy since it was one of my first Rust projects and it's not built on a framework or anything.<br>
(if I continue development I would probably want to overhaul how it's structured)<br>
I haven't worked on it for a while but the folks working on [Astratomic](https://github.com/spicylobstergames/astratomic) were interested in seeing how I did certain things.

Some features:
- Chunk based infinite world
- Multithreaded sand sim (uses unsafe, questionable soundness)
- Multithreaded particles (uses unsafe, probably unsound)
- Rapier2d rigidbodies (with partially simulated pixels)
- GPU compute colored lighting (Terraria style)
- Partially multithreaded world/structure generation system inspired by Minecraft's
- Very basic entity system using specs ECS

https://github.com/PieKing1215/FallingSandEngine/assets/13819558/a76c127c-236b-4526-8666-2823cc6e162c

Some points of interest:
- [world.rs](https://github.com/PieKing1215/FallingSandEngine/blob/master/fs_common/src/game/common/world/world.rs#L400): World tick function
  - <details><summary>Pseudocode for rigidbody/entity/sand tick sequence</summary>
    
    ```
    // fill rigidbody hitboxes in world with dummy pixels
    for each rigidbody
        for each point in rigidbody
            calculate world position for the point
            set world pixel at that position to dummy "object" type (acts like solid)
            if there was a sand pixel there
                make it a particle or displace it
                apply impulse to rigidbody at the point
    
    // fill entity hitboxes in world with dummy pixels
    for each entity
        for each point in entity
            calculate world position for the point
            if world pixel at that position is air:
                set world pixel at that position to dummy "object" type
    
    tick pixel (sand) simulation for chunks
    
    tick particle simulation
    
    // clear dummy pixels from entity hitboxes
    for each entity
        for each point in entity
            calculate world position for the point
            if world pixel at that position is dummy "object" type:
                set world pixel at that position to air
    
    tick entities
    
    // clear dummy pixels from rigidbody hitboxes
    for each rigidbody
        for each point in rigidbody
            calculate world position for the point
            if world pixel at that position is dummy "object" type:
                set world pixel at that position to air
    
    tick pixel (sand) simulation for rigidbodies
    
    update chunk collision
    ```
    </details> 
- [simulator.rs](https://github.com/PieKing1215/FallingSandEngine/blob/master/fs_common/src/game/common/world/simulator.rs#L445): Sand simulation for chunks & RigidBodies
  - See also [this issue thread](https://github.com/PieKing1215/FallingSandSurvival/issues/4) which describes some of the techniques I used
- [chunk_handler.rs](https://github.com/PieKing1215/FallingSandEngine/blob/master/fs_common/src/game/common/world/chunk_handler.rs#L104): Chunk handler tick function (chunk loading/unloading/etc)
- [lighting_prep.comp](https://github.com/PieKing1215/FallingSandEngine/blob/master/gamedir/assets/data/shaders/lighting_prep.comp)/[lighting_propagate.comp](https://github.com/PieKing1215/FallingSandEngine/blob/master/gamedir/assets/data/shaders/lighting_propagate.comp)/[chunk_light.frag](https://github.com/PieKing1215/FallingSandEngine/blob/master/gamedir/assets/data/shaders/chunk_light.frag): Lighting shaders
- [particle.rs](https://github.com/PieKing1215/FallingSandEngine/blob/master/fs_common/src/game/common/world/particle.rs#L149): Particles
- [gen/](https://github.com/PieKing1215/FallingSandEngine/tree/master/fs_common/src/game/common/world/gen): World gen systems
  - Conceptually heavily inspired by Minecraft's biome/feature/structure generation system
    - https://minecraft.wiki/w/Custom_world_generation
    - https://www.youtube.com/watch?v=CSa5O6knuwI&t=1342s
  - See also [this article](https://accidentalnoise.sourceforge.net/minecraftworlds.html) which describes a cave generation technique that mine is based off of
- [gamedir/assets/data/structure/](https://github.com/PieKing1215/FallingSandEngine/tree/master/gamedir/assets/data/structure): Worldgen structure data

## Download

Automatic builds for Windows x64 can be found here (requires GitHub account to download): https://github.com/PieKing1215/FallingSandEngine/actions/workflows/autobuild.yml

## Building

Install rust/cargo

Clone the repo

To run locally you should be able to just do `cargo run`/`cargo run --release`<br>
You can also add `-- -d` to enable debug UI<br>
(there's also a `profile` feature which enables profiling with Tracy)

I haven't built it to linux in a while but it should work

There's also a bash script which can bundle everything needed to run the game into the package/ folder (builds for release, copies assets, and generates a dependency licences file)<br>
It requires cargo-lichking: `cargo install cargo-lichking`<br>
To run: `sh package.sh`
