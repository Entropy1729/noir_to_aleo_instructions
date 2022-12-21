use indexmap::IndexMap;
use noirc_abi::AbiFEType;
use noirc_frontend::{
    BinaryOpKind, BlockExpression, ConstrainStatement, ExpressionKind, Ident, NoirFunction,
    ParsedModule,
    Pattern::{self, Identifier, Mutable, Struct, Tuple},
    Statement, UnresolvedType,
};
use std::{ffi::OsString, path::Path};

mod not_nargo;
use not_nargo::into_parsed_program;

const ALEO_BUILD_DIR: &str = "build/aleo";

fn compile_to_aleo_instructions<P: AsRef<Path>>(program_dir: P) {
    let (program_name, noir_ast) = into_parsed_program(program_dir);
    let compiled_aleo_program = compile_program(&program_name, noir_ast);
    build_aleo_program(program_name, compiled_aleo_program);
}

fn compile_program(program_name: &OsString, noir_ast: ParsedModule) -> String {
    let mut aleo_program = String::new();
    let aleo_program_name = format!("program {}.aleo;", program_name.to_str().unwrap());
    aleo_program.push_str(&aleo_program_name);
    push_new_line(&mut aleo_program);
    push_new_line(&mut aleo_program);

    for function in noir_ast.functions {
        compile_function(&function, &mut aleo_program);
    }

    aleo_program
}

fn push_new_line(aleo_program: &mut String) {
    aleo_program.push('\n');
}

fn compile_function(function: &NoirFunction, aleo_program: &mut String) {
    let mut register_registry: IndexMap<Option<String>, String> = IndexMap::new();
    // This counter is used for intermediate variables.
    // Register counter will be increased every time a new register is created,
    // that should include the case of intermediate register creation.
    let mut register_count = 0_u32;
    let function_definition = to_aleo_function_definition(function.name());
    aleo_program.push_str(&function_definition);
    push_new_line(aleo_program);
    /* Inputs */
    for (parameter, unresolved_type, visibility) in function.parameters() {
        let input_line = to_aleo_input_line(
            parameter,
            unresolved_type,
            *visibility,
            &mut register_count,
            &mut register_registry,
        );
        aleo_program.push_str(&input_line);
    }
    /* Body (a.k.a. operations) */
    let function_def = function.def();
    let BlockExpression(mut body) = function_def.body.clone();
    body.reverse();
    while let Some(statement) = body.pop() {
        let statement_line =
            to_aleo_operation_line(&statement, &mut register_count, &mut register_registry);
        aleo_program.push_str(&statement_line);
    }
    let output_type = to_aleo_type(&function_def.return_type);
    let output_visibility = to_aleo_visibility(function_def.return_visibility);
    let (_, output_register) = register_registry.last().unwrap();
    aleo_program.push_str(&format!(
        "\toutput {} as {}.{};\n",
        output_register, output_type, output_visibility
    ));
}

fn to_aleo_function_definition(function_name: &str) -> String {
    format!("function {}:", function_name)
}

fn to_aleo_input_line(
    parameter: &Pattern,
    unresolved_type: &UnresolvedType,
    visibility: AbiFEType,
    register_count: &mut u32,
    register_registry: &mut IndexMap<Option<String>, String>,
) -> String {
    match parameter {
        Identifier(Ident(ident)) => {
            let register = to_aleo_register(*register_count);
            let register_type = to_aleo_type(unresolved_type);
            let visibility = to_aleo_visibility(visibility);

            register_registry.insert(Some(ident.contents.clone()), register.clone());
            *register_count += 1;

            format!("\tinput {register} as {register_type}.{visibility};\n")
        }
        Mutable(_, _) => todo!(),
        Tuple(_, _) => todo!(),
        Struct(_, _, _) => todo!(),
    }
}

fn to_aleo_register(register_number: u32) -> String {
    format!("r{register_number}")
}

fn to_aleo_type(unresolved_type: &UnresolvedType) -> String {
    match unresolved_type {
        UnresolvedType::FieldElement(_) => "field".to_owned(),
        UnresolvedType::Array(_, _) => todo!(),
        UnresolvedType::Integer(_, signedness, num_bits) => match signedness {
            noirc_frontend::Signedness::Signed => format!("i{}", num_bits),
            noirc_frontend::Signedness::Unsigned => format!("u{}", num_bits),
        },
        UnresolvedType::Bool(_) => todo!(),
        UnresolvedType::Unit => todo!(),
        UnresolvedType::Named(_, _) => todo!(),
        UnresolvedType::Tuple(_) => todo!(),
        UnresolvedType::Unspecified => todo!(),
        UnresolvedType::Error => todo!(),
    }
}

fn to_aleo_visibility(visibility: AbiFEType) -> String {
    match visibility {
        AbiFEType::Public => "public".to_owned(),
        AbiFEType::Private => "private".to_owned(),
    }
}

// TODO: register_count will be useful for intermediate variables.
fn to_aleo_operation_line(
    statement: &Statement,
    register_count: &mut u32,
    register_registry: &mut IndexMap<Option<String>, String>,
) -> String {
    match statement {
        Statement::Let(_) => todo!(),
        Statement::Constrain(ConstrainStatement(expression)) => {
            // It is tempting to abstract this using handle_expression, but it
            // should be noticed that the constrain statement expression is not
            // a regular expression.
            match &expression.kind {
                ExpressionKind::Infix(infix_expression) => {
                    let left_operand = handle_expression(
                        &infix_expression.lhs.kind,
                        register_count,
                        register_registry,
                    );
                    let right_operand = handle_expression(
                        &infix_expression.rhs.kind,
                        register_count,
                        register_registry,
                    );
                    // TODO: Abstract this into a function
                    let operator = match &infix_expression.operator.contents {
                        BinaryOpKind::Equal => "assert.eq",
                        BinaryOpKind::NotEqual => "assert.neq",
                        _ => todo!(),
                    };
                    format!("\t{operator} {left_operand} {right_operand};\n")
                }
                _ => todo!(),
            }
        }
        Statement::Expression(expression) => {
            handle_expression(&expression.kind, register_count, register_registry)
        }
        Statement::Assign(_) => todo!(),
        Statement::Semi(_) => todo!(),
        Statement::Error => todo!(),
    }
}

fn handle_expression(
    expression: &ExpressionKind,
    register_count: &mut u32,
    register_registry: &mut IndexMap<Option<String>, String>,
) -> String {
    match &expression {
        ExpressionKind::Ident(_) => todo!(),
        ExpressionKind::Literal(_) => todo!(),
        ExpressionKind::Block(_) => todo!(),
        ExpressionKind::Prefix(_) => todo!(),
        ExpressionKind::Index(_) => todo!(),
        ExpressionKind::Call(_) => todo!(),
        ExpressionKind::MethodCall(_) => todo!(),
        ExpressionKind::Constructor(_) => todo!(),
        ExpressionKind::MemberAccess(_) => todo!(),
        ExpressionKind::Cast(_) => todo!(),
        ExpressionKind::Infix(infix_expression) => {
            let left_operand: String = handle_expression(
                &infix_expression.lhs.kind,
                register_count,
                register_registry,
            );
            let right_operand: String = handle_expression(
                &infix_expression.rhs.kind,
                register_count,
                register_registry,
            );
            let operator = to_aleo_operator(&infix_expression.operator.contents);
            let destination_register = to_aleo_register(*register_count);
            register_registry.insert(None, destination_register.clone());
            *register_count += 1;
            format!("\t{operator} {left_operand} {right_operand} into {destination_register};\n")
        }
        ExpressionKind::For(_) => todo!(),
        ExpressionKind::If(_) => todo!(),
        ExpressionKind::Path(path) => {
            // Probably important later.
            let _path_kind = path.kind;
            let Ident(ident) = path.segments.first().unwrap();
            register_registry
                .get(&Some(ident.contents.clone()))
                .unwrap()
                .clone()
        }
        ExpressionKind::Tuple(_) => todo!(),
        ExpressionKind::Error => todo!(),
    }
}

fn build_aleo_program(mut program_name: OsString, compiled_program: String) {
    let mut aleo_path = std::env::current_dir().unwrap();
    aleo_path.push(ALEO_BUILD_DIR);
    program_name.push(".aleo");
    aleo_path.push(program_name.clone());
    std::fs::create_dir_all(aleo_path.parent().unwrap()).unwrap();

    let mut aleo_file = std::fs::File::create(aleo_path).unwrap();
    std::io::Write::write_all(&mut aleo_file, compiled_program.as_bytes()).unwrap();
}

// fn to_aleo_operator(operator: &BinaryOpKind) -> &str {
//     match operator {
//         BinaryOpKind::Add => "add",
//         BinaryOpKind::Subtract => "sub",
//         BinaryOpKind::Multiply => "mul",
//         BinaryOpKind::Divide => "div",
//         BinaryOpKind::Equal => "is.eq",
//         BinaryOpKind::NotEqual => "is.neq",
//         BinaryOpKind::Less => "lt",
//         BinaryOpKind::LessEqual => "lte",
//         BinaryOpKind::Greater => "gt",
//         BinaryOpKind::GreaterEqual => "gte",
//         BinaryOpKind::And => "and",
//         BinaryOpKind::Or => "or",
//         BinaryOpKind::Xor => "xor",
//         BinaryOpKind::ShiftRight => "shr",
//         BinaryOpKind::ShiftLeft => "shl",
//         BinaryOpKind::Modulo => "mod",
//     }
// }

// TODO: Make a CLI app.
fn main() {}

#[cfg(test)]
mod tests {
    use crate::{compile_program, compile_to_aleo_instructions, not_nargo::into_parsed_program};

    const TEST_DATA_DIR: &str = "tests/";

    #[test]
    fn test_compile_noir_hello_world_to_aleo_instructions() {
        let mut program_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        program_dir.push(&format!("{TEST_DATA_DIR}/hello_world_noir_crate"));
        compile_to_aleo_instructions(program_dir);
    }

    #[test]
    fn test_add() {
        let mut program_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        program_dir.push(&format!("{TEST_DATA_DIR}/add"));
        let (program_name, noir_ast) = into_parsed_program(program_dir);
        let expected_compiled_program = "program main.nr.aleo;\n\nfunction add:\n\tinput r0 as u32.private;\n\tinput r1 as u32.private;\n\tadd r0 r1 into r2;\n\toutput r2 as u32.private;\n";

        let compiled_program = compile_program(&program_name, noir_ast);

        assert_eq!(compiled_program, expected_compiled_program);
    }
}
