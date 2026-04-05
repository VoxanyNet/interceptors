use strum::{Display, EnumIter};

use crate::{base_prop::BaseProp, props::wooden_box::wooden_box::WoodenBox};

pub mod wooden_box;

// There are many layers to emulating objects in rust
// Dynamic dispatch (different functions called depending on the underlying type)
// A registry of all implementors 
// An interface that the different types implement so that can all be called the same way
// A base struct that provides the basic functionality for implementors (like a base class)
// Delegating methods that superclasses dont implement to the base class
// Superclasses can decide if they want to use the base class at all and can just implement the trait manually
// Types must be stored on the heap because they will be of different sizes

