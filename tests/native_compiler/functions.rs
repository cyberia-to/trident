use super::support::{self, Result};

#[test]
fn native_job_emits_reusable_functions_with_fresh_frames_and_ordered_arguments() {
    support::worker(|| {
        for (source, expected) in [
            ("fn helper()->Field{7} fn main()->Field{9}",9),
            ("fn main()->Field{helper()} fn helper()->Field{7}",7),
            ("fn f(a:Field,b:Field)->Field{10*a+b} fn main()->Field{f(2,3)}",23),
            ("fn f(a:Field,b:Field)->Field{10*a+b} fn g(x:Field)->Field{x+1} fn main()->Field{f(g(2),g(3))}",34),
            ("fn f(x:Field)->Field{x+1} fn main()->Field{f(f(f(2)))}",5),
            ("fn f()->Field{7} fn main()->Field{let f=9 f()+f}",16),
            ("fn f(x:Field)->Field{let mut y=x y=y+1 return y} fn main()->Field{let mut x=3 x=f(x) f(x)+x}",9),
            ("fn f(x:Field)->Field{if x==2{return 7} 9} fn main()->Field{let x=3 f(2)+f(x)+x}",19),
            ("fn f(x:Bool)->Bool{if x{return false} true} fn main()->Field{if f(false){7}else{9}}",7),
            ("fn f(x:Field,x:Bool)->Bool{x} fn main()->Field{if f(4,true){7}else{9}}",7),
            ("fn f()->Field{7} fn f()->Bool{true} fn main()->Field{if f(){7}else{9}}",7),
            ("fn f(){} fn main()->Field{f() 7}",7),
            ("fn f(){return} fn g(){return f()} fn main()->Field{g() 7}",7),
            ("fn f(){} fn main()->Field{let mut x=f() x=f() if x==f(){7}else{9}}",7),
            ("fn f(){if true{7} let x=8} fn main()->Field{f() 7}",7),
            ("fn main(x:Field)->Field{x} fn main()->Field{9}",9),
            ("fn f()->Field{f()} fn f()->Field{7} fn main()->Field{f()}",7),
            ("fn f(a:Field,b:Field,c:Field)->Field{let d=9 let e=8 a+10*b+100*c+d+e} fn main()->Field{f(1,2,3,)}",338),
            ("fn f(x:Field)->Field{x} fn main()->Field{f((2+3)*4)}",20),
        ] {
            let source = format!("program sample {source}");
            let result = support::compile(source.as_bytes());
            assert_eq!(support::value(result), expected, "{source}");
            assert_eq!(support::rust_value(&source), expected, "seed: {source}");
        }
    });
}

fn artifact(source: &str) -> Vec<u8> {
    match support::compile(source.as_bytes()) {
        Result::Program { bytes, .. } => bytes,
        other => panic!("{source}: {other:?}"),
    }
}

#[test]
fn function_table_identity_ignores_declaration_order_and_unreachable_bodies() {
    support::worker(|| {
        let canonical = artifact("program sample fn f(x:Field)->Field{x+1} fn g(x:Field)->Field{f(x)*2} fn main()->Field{g(3)}");
        assert_eq!(canonical, artifact("program sample fn main()->Field{g(3)} fn g(x:Field)->Field{f(x)*2} fn f(x:Field)->Field{x+1}"));
        assert_eq!(canonical, artifact("program sample fn unused(x:Field)->Field{x+9} fn f(x:Field)->Field{x+1} fn g(x:Field)->Field{f(x)*2} fn main()->Field{g(3)}"));
        assert_eq!(
            artifact("program sample fn main()->Field{7}"),
            artifact("program sample fn f()->Field{9} fn main()->Field{7}")
        );
    });
}

#[test]
fn function_job_rejects_cycles_and_checks_every_body_and_call() {
    support::worker(|| {
        for source in [
            "fn main()->Field{main()}",
            "fn f()->Field{g()} fn g()->Field{f()} fn main()->Field{7}",
            "fn f()->Field{if false{return f()} 7} fn main()->Field{7}",
            "fn f()->Field{7} fn f()->Field{f()} fn main()->Field{7}",
            "fn f()->Bool{7} fn main()->Field{9}",
            "fn f()->Field{7} fn main()->Field{f(1)}",
            "fn f(x:Field)->Field{x} fn main()->Field{f()}",
            "fn f(x:Bool)->Bool{x} fn main()->Field{if f(1){7}else{9}}",
            "fn f(x:Field)->Field{x=2 x} fn main()->Field{f(1)}",
            "fn main()->Field{missing()}",
            "fn f(){if false{7}} fn main()->Field{7}",
            "fn main(x:Bool)->Field{x} fn main()->Field{9}",
        ] {
            support::error(format!("program sample {source}").as_bytes(), 5);
        }
    });
}
