#[derive(Clone, Copy)]
pub enum Block {
    Air,
    Stone,
}

#[derive(Default)]
pub struct Faces {
    pub front: Vec<(cgmath::Vector3<u8>, Block)>,
    pub back: Vec<(cgmath::Vector3<u8>, Block)>,
    pub left: Vec<(cgmath::Vector3<u8>, Block)>,
    pub right: Vec<(cgmath::Vector3<u8>, Block)>,
    pub top: Vec<(cgmath::Vector3<u8>, Block)>,
    pub bottom: Vec<(cgmath::Vector3<u8>, Block)>,
}

pub struct Chunk {
    pub blocks: Box<[[[Block; 16]; 16]; 16]>,
}

impl Chunk {
    pub fn get_block(&self, x: u8, y: u8, z: u8) -> Option<Block> {
        self.blocks
            .get(x as usize)
            .and_then(|blocks| blocks.get(y as usize))
            .and_then(|blocks| blocks.get(z as usize))
            .copied()
    }

    pub fn generate_faces(&self) -> Faces {
        let mut faces = Faces::default();
        for x in 0u8..16 {
            for y in 0u8..16 {
                for z in 0u8..16 {
                    let position = cgmath::vec3(x, y, z);
                    let block = self.blocks[x as usize][y as usize][z as usize];
                    if !matches!(block, Block::Air) {
                        if x.checked_add(1)
                            .and_then(|x| self.get_block(x, y, z))
                            .map_or(true, |block| matches!(block, Block::Air))
                        {
                            faces.front.push((position, block));
                        }
                        if x.checked_sub(1)
                            .and_then(|x| self.get_block(x, y, z))
                            .map_or(true, |block| matches!(block, Block::Air))
                        {
                            faces.back.push((position, block));
                        }
                        if y.checked_add(1)
                            .and_then(|y| self.get_block(x, y, z))
                            .map_or(true, |block| matches!(block, Block::Air))
                        {
                            faces.top.push((position, block));
                        }
                        if y.checked_sub(1)
                            .and_then(|y| self.get_block(x, y, z))
                            .map_or(true, |block| matches!(block, Block::Air))
                        {
                            faces.bottom.push((position, block));
                        }
                        if z.checked_add(1)
                            .and_then(|z| self.get_block(x, y, z))
                            .map_or(true, |block| matches!(block, Block::Air))
                        {
                            faces.right.push((position, block));
                        }
                        if z.checked_sub(1)
                            .and_then(|z| self.get_block(x, y, z))
                            .map_or(true, |block| matches!(block, Block::Air))
                        {
                            faces.left.push((position, block));
                        }
                    }
                }
            }
        }
        faces
    }
}
