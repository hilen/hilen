use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, Token,
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
};

pub(crate) fn async_link_button_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ButtonArgs);

    let button_path = args.button;
    let method = args.method;

    let expanded = if let Some(arg) = args.arg {
        quote! {
            #button_path.on_tap(move || {
                hilen::dispatch::spawn(async move {
                    use hilen::ui::AlertErr;
                    self.#method(#arg).await.alert_err();
                });
            });
        }
    } else {
        quote! {
            #button_path.on_tap(move || {
                hilen::dispatch::spawn(async move {
                    use hilen::ui::AlertErr;
                    self.#method().await.alert_err();
                });
            });
        }
    };

    TokenStream::from(expanded)
}

struct ButtonArgs {
    button: Expr,
    method: Ident,
    arg:    Option<Expr>,
}

impl Parse for ButtonArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let button: Expr = input.parse()?;
        let _comma1: Token![,] = input.parse()?;
        let method: Ident = input.parse()?;

        let arg = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(ButtonArgs { button, method, arg })
    }
}
