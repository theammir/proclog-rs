#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    #[test]
    fn two_args() {
        #[log_attrib::log(INFO)]
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }
        assert_eq!(add(2, 2), 4);
        assert_eq!(add(-10, 15), 5);
    }

    #[test]
    fn no_args() {
        #[log_attrib::log(DEBUG)]
        fn foo() {}
        foo();
    }

    #[test]
    fn unsafe_generic() {
        #[log_attrib::log(DEBUG)]
        unsafe fn unsafe_unwrap<T: Debug, E: Debug>(a: Result<T, E>) -> T {
            unsafe { a.unwrap_unchecked() }
        }
        assert_eq!(unsafe { unsafe_unwrap::<_, ()>(Ok(2)) }, 2);
    }

    #[test]
    fn try_op() {
        #[log_attrib::log(ERROR)]
        fn try_inner(o: Option<i32>) -> Option<i32> {
            Some(o? + 2)
        }
        assert_eq!(try_inner(None), None);
    }

    #[test]
    fn self_receiver_ref() {
        #[derive(Debug)]
        struct A(u8);

        impl A {
            #[log_attrib::log(INFO)]
            fn foo(&self, a: u8) -> u8 {
                self.0 + a
            }
        }
        let a = A(10);
        assert_eq!(a.foo(5), 15);
    }

    #[test]
    fn self_receiver_mut() {
        #[derive(Debug)]
        struct A;

        impl<'a> A {
            #[log_attrib::log(INFO)]
            fn foo(&mut self, s: &'a str) -> &'a str {
                s
            }
        }
        let mut a = A;
        assert_eq!(a.foo("hi!"), "hi!");
    }

    #[test]
    fn asynchronous() {
        #[log_attrib::log(INFO)]
        async fn square(a: u32) -> u32 {
            a * a
        }
        assert_eq!(futures::executor::block_on(square(3)), 9);
    }
}
