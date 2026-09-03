use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Data, DeriveInput, Field, Fields, GenericParam, Ident, parse::Parser, parse_macro_input, spanned::Spanned,
};

/// The two game object kinds the macros build. A level is the 2D
/// physics world, a scene the 3D one, and both get the same shape: an
/// injected base, the trait impls, and a test ctor behind a feature.
struct Kind {
    module:      &'static str,
    base:        &'static str,
    object:      &'static str,
    internal:    &'static str,
    setup:       &'static str,
    registrable: &'static str,
    register:    &'static str,
    /// Whether the crate feature that turns the test ctor on is set.
    tests:       bool,
}

#[proc_macro_attribute]
pub fn level(_args: TokenStream, stream: TokenStream) -> TokenStream {
    expand(
        stream,
        &Kind {
            module:      "level",
            base:        "LevelBase",
            object:      "Level",
            internal:    "LevelInternal",
            setup:       "LevelSetup",
            registrable: "LevelRegistrable",
            register:    "register_if_level_test",
            tests:       cfg!(feature = "level-tests"),
        },
    )
}

#[proc_macro_attribute]
pub fn scene(_args: TokenStream, stream: TokenStream) -> TokenStream {
    expand(
        stream,
        &Kind {
            module:      "scene",
            base:        "SceneBase",
            object:      "Scene",
            internal:    "SceneInternal",
            setup:       "SceneSetup",
            registrable: "SceneRegistrable",
            register:    "register_if_scene_test",
            tests:       cfg!(feature = "scene-tests"),
        },
    )
}

#[allow(clippy::too_many_lines)]
fn expand(stream: TokenStream, kind: &Kind) -> TokenStream {
    let mut stream = parse_macro_input!(stream as DeriveInput);

    let Data::Struct(data) = &mut stream.data else {
        panic!("`{}` macro has to be used with structs", kind.module)
    };

    let name = &stream.ident;

    let module = Ident::new(kind.module, name.span());
    let base = Ident::new(kind.base, name.span());
    let object = Ident::new(kind.object, name.span());
    let internal = Ident::new(kind.internal, name.span());
    let setup = Ident::new(kind.setup, name.span());
    let registrable = Ident::new(kind.registrable, name.span());
    let register = Ident::new(kind.register, name.span());
    let base_field = format_ident!("__{}_base", kind.module);

    let generics = &stream.generics;

    let type_param_names: Vec<_> = generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(type_param) => Some(type_param.ident.clone()),
            GenericParam::Const(const_param) => Some(const_param.ident.clone()),
            GenericParam::Lifetime(_) => None,
        })
        .collect();

    let type_params = quote_spanned! {stream.generics.span()=>
        #(#type_param_names),*
    };

    let Fields::Named(fields) = &mut data.fields else {
        panic!("No named fields");
    };

    fields.named.insert(
        0,
        Field::parse_named
            .parse2(quote! { #base_field: hilen::#module::#base })
            .expect("parse2(quote! { base field })"),
    );

    // A ctor names one concrete type, so a generic object gets no marker
    // and `impl LevelTest` or `impl SceneTest` on it fails to compile
    // instead of never running.
    let test_registration = if generics.params.is_empty() {
        let ctor = if kind.tests {
            let register_fn = format_ident!("__register_{}_test_{name}", kind.module);
            quote! {
                #[hilen::__internal_macro_deps::ctor::ctor(unsafe, crate_path = hilen::__internal_macro_deps::ctor)]
                fn #register_fn() {
                    hilen::#module::#register::<#name>(file!());
                }
            }
        } else {
            quote!()
        };
        quote! {
            impl hilen::#module::#registrable for #name {}
            #ctor
        }
    } else {
        quote!()
    };

    quote! {
        #stream

        impl #generics hilen::#module::#object for #name <#type_params> { }

        #test_registration

        impl #generics hilen::#module::#internal for #name <#type_params> {
            fn __internal_setup(&self) {
                use hilen::#module::#setup;
                let mut object = hilen::refs::weak_from_ref(self);
                if object.needs_physics() {
                    object.init_physics();
                }
                object.setup();
            }

            fn __internal_update(&self, frame_time: f32) {
                use hilen::#module::#object;
                use hilen::#module::#setup;
                let mut object = hilen::refs::weak_from_ref(self);
                let steps = if object.has_physics() {
                    hilen::#module::#base::PHYSICS_SUBSTEPS
                } else {
                    1
                };
                let dt = frame_time / steps as f32;
                for _ in 0..steps {
                    object.update(dt);
                    object.update_physics(dt);
                }
            }
        }

        impl #generics hilen::refs::AsAny for #name <#type_params> {
            fn as_any(&self) -> &dyn std::any::Any {
               self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
               self
            }

            fn into_any_box(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }

        impl #generics std::ops::Deref for #name <#type_params> {
            type Target = hilen::#module::#base;
            fn deref(&self) -> &hilen::#module::#base {
                &self.#base_field
            }
        }
        impl #generics std::ops::DerefMut for #name <#type_params>  {
            fn deref_mut(&mut self) -> &mut hilen::#module::#base {
                &mut self.#base_field
            }
        }
    }
    .into()
}
