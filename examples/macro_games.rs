//#![feature(trace_macros)]

#[macro_export]
macro_rules! foo {
    ( $prefix:ident, $( $x:ident ),* ) => {
        $(
            #[no_mangle]
            extern fn $prefix$x() { let i = $x; }
        )*
    };
}

//trace_macros!(true);
//foo![prefix, alloc_set, alloc_get, alloc_del];
//trace_macros!(false);

fn bar() {
    let v = vec![1,2,3];
    stringify!(1);
}

fn foo1() {
    #[no_mangle]
    extern fn foo1a() {}
}

fn foo2() {
//    #[no_mangle]
    extern fn foo1a() {}
}

#[macro_export]
macro_rules! vec {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec = Vec::new();
            $(
                temp_vec.push($x);
            )*
            temp_vec
        }
    };
}

