use clap::ValueEnum;


#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum PrefabType {
    Decoration,
    Prop,
}