[!IMPORTANT]
This is a Student Project!

# Pulse Monitor Project

This project is divided into two parts: [the monitor](#monitor) and the [desktop application](#application)

## Monitor

Author: Cole

## Application

This is the Pulse [Monitor Desktop Application](./desktop-application/) part of the README!

We will be using Iced for Rust as our way to create and modify our application and graphical user interface. While we could go for something more fun like: [Ratatui](https://ratatui.rs/), I don't think people will be that willing to use a Terminal User Interface even if it is graphical.

[!NOTE]
There is a git ignore in the desktop application for a file called `/target`. You will not see this file as it has been ignored by git. The `target` file is created when you build the application with `cargo run` or `cargo build`. The file will contain a lot of primaraly usless compiler stuff. When running `cargo build` a target directory will be created and under that directory will be another directory called `debug`. This contains the bianary application of the project compiled to have as much debugg information as possible. But if you run `cargo build --release` then a `release` directory will appear and this is your release code on optimization level 2.

### Needs

What are the basic needs of the application:

* Create a responsive ( resisable, minimizable, and tiltable ) window

* Display heart rate in a rsponsive way

* Be not bad to look at

### Wants

Would be nice but arn't required for the project

* Show a graph of heart rate over time

* Show diffrent ( previous ) activities

* Settings that controll how the content looks
