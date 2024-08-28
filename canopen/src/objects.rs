// pub enum ObjectAccess {
 
// }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectCode {   
    Null = 0,
    Domain = 2,
    DefType = 5,
    DefStruct = 6,
    Var = 7,
    Array = 8,
    Record = 9,
}   

pub struct Object {
    pub index: u16,
    pub number_of_subs: u8,
    pub highest_sub: u8,
    pub object_code: ObjectCode,
}


pub struct ObjectDict {

}