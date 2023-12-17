fn main() -> anyhow::Result<()> {
    pollster::block_on(block_game::run())
}
