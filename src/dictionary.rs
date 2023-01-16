use crate::raw;


#[derive(Debug, PartialEq)]
pub struct Dictionary {
    ctx: *mut raw::RedisModuleCtx,
    inner: *mut raw::RedisModuleDict,
}

impl Dictionary {
    pub fn new(ctx: *mut raw::RedisModuleCtx) -> Self {
        let inner = unsafe { raw::RedisModule_CreateDict.unwrap()(ctx) };
        Self { ctx, inner }
    }

    // pub fn replace(&mut self,  ctx: *mut raw::RedisModuleCtx) -> Self {
    //     let inner = unsafe { raw::RedisModule_DictReplace.unwrap()(ctx) };
    //     Self { ctx, inner }
    // }
}

impl Drop for Dictionary {
    fn drop(&mut self) {
        unsafe {
            raw::RedisModule_FreeDict.unwrap()(self.ctx, self.inner);
        }
    }
}

// impl Display for Dictionary {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.to_string_lossy())
//     }
// }