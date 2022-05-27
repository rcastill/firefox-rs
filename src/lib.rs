
#[derive(thiserror::Error, Debug)]
pub enum Error {

}

// Firefox Result
pub type FFResult<T> = Result<T, Error>;

#[derive(Debug)]
pub struct Tab {

}

impl Tab {
    fn focus(&self) -> FFResult<()> {
        todo!()
    }
}

pub fn list_tabs() -> FFResult<Vec<Tab>> {
    todo!()
}



#[cfg(test)]
mod tests {
    
}
