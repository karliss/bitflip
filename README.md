# Bitflip

Unofficial implementation for programming/puzzle game [Rogue Bit](https://roguebit.bigosaur.com/)

## Why make this

I like programming games so when I saw this one I knew that I will run it in proper terminal emulator. Reimplementing the game seeming seemed like a good exercise for practising Rust. 

## FAQ

* Pressing New game ends the program - read next question.
* Where are the levels?  - You can play the level from original game by copying ram.txt and ram2.txt to resources/levels/rb folder.
Loading savefile or custom level from .storage file partially works using play commandline option.
* How to build this? - [Read the manual](https://doc.rust-lang.org/1.27.2/book/second-edition/ch01-00-getting-started.html)
* What are the supported operating systems? - Linux and macOS should work. Windows doesn't work due to lack of support from terminal library.

## supported level formats
* single .txt file
* .yaml file describing multipage level with customized properties see levels/rb/config.yaml as example and structure definitions in code
* .storage file from savefile or custom level - partially working. Not all files work.

## State of project

* Is this good code? No!
* Is this project maintained? No, I might add some functionality while trying out a new library.
