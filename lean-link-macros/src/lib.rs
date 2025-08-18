use proc_macro::TokenStream;
use quote::quote;
use syn::{ExprArray, ItemFn, parse_macro_input, parse_quote};

#[proc_macro_attribute]
pub fn web_main(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(input as ItemFn);

    let sig = &mut input_fn.sig;
    if sig.ident != "main" {
        panic!("This macro can only be applied to the `main` function")
    }

    input_fn
        .attrs
        .insert(0, syn::parse_quote! { #[actix_web::main] });

    sig.output = syn::parse_quote! { -> std::io::Result<()> };

    let original_block = input_fn.block;
    input_fn.block = parse_quote! {{
        tracing_subscriber::fmt().init();
        #original_block
    }};
    TokenStream::from(quote! {
        #input_fn
    })
}

#[proc_macro_attribute]
pub fn tokio_main(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(input as ItemFn);

    let sig = &mut input_fn.sig;
    if sig.ident != "main" {
        panic!("This macro can only be applied to the `main` function")
    }

    input_fn
        .attrs
        .insert(0, syn::parse_quote! { #[tokio::main] });

    sig.output = syn::parse_quote! { -> std::io::Result<()> };

    let original_block = input_fn.block;
    input_fn.block = parse_quote! {{
        tracing_subscriber::fmt().init();
        #original_block
    }};
    TokenStream::from(quote! {
        #input_fn
    })
}

#[proc_macro]
pub fn new_actix_app(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        actix_web::App::new().wrap(tracing_actix_web::TracingLogger::default())
    };
    expanded.into()
}

#[proc_macro]
pub fn config_db(input: TokenStream) -> TokenStream {
    let migrations = parse_macro_input!(input as ExprArray);

    let expanded = quote! {
        struct Migrator;
        #[async_trait::async_trait]
        impl sea_orm_migration::MigratorTrait for Migrator {
            fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
                let mut migrations = Vec::<Box<dyn sea_orm_migration::MigrationTrait>>::new();
                migrations.push(Box::new(
                    lean_link::database::migrator::m20250814_000001_create_tables::Migration,
                ));

                #migrations.into_iter().for_each(|m| {
                    migrations.push(Box::new(m));
                });

                migrations
            }
        }

        async fn setup_db(conn: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
            use sea_orm_migration::MigratorTrait;
            Migrator::up(conn, None).await
        }
    };

    TokenStream::from(expanded)
}
