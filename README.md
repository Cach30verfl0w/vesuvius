# Vesuvius
Modern Day Strategy Game, written in Rust with Vulkan. This project is an educational project to learn Shader Programming and how to work with Vulkan. Many mechanics in this game are inspired by the WW2 Strategy Game [Hearts of Iron IV](https://en.wikipedia.org/wiki/Hearts_of_Iron_IV).

This game contains the following scenarios:
- 1991: Dissolvement of the Soviet Union
- 2001: Begin of the War against Terrorism (9/11)

## Engine and technical detail
The full game is written without any external game engine or other high-level Vulkan abstractions. As Vulkan wrapper for Rust, I used [ash-rs](https://github.com/ash-rs/ash). The game's code is not the best, because I was never really working with Vulkan before. I have to thank [BeastLe9enD](https://github.com/BeastLe9enD) for the idea to write a game in Vulkan and for some of his knowledge in Graphics Programming.

