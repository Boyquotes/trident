use crate::idl_detect;

pub fn detect_in_idl(idl: idl_detect::Idl) {
    println!("Looking for expression:\n");

    for program in idl.programs.into_iter() {
        for function in program.functions.into_iter() {
            for stmt in function.block.stmts.into_iter() {
                match stmt {
                    syn::Stmt::Semi(syn::Expr::AssignOp(_expression), _semi) => {
                        println!("Found +=")
                    }
                    syn::Stmt::Semi(syn::Expr::Assign(expression), _semi) => {
                        if let syn::Expr::Binary(_binary_exp) = *expression.right {
                            println!("Found add")
                        }
                    }
                    _ => (),
                };
            }
        }
    }
}
