#[test]
fn work(){
    assert!(true);
}

use std::process::Command;

#[test]
fn runs() {
    let res = Command::new("powershell")
        .args(["ls"])
        .output();
    println!("{:?}", res);
    assert!(res.is_ok());
}