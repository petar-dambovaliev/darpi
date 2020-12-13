// #![feature(prelude_import)]
// #[prelude_import]
// use std::prelude::v1::*;
// #[macro_use]
// extern crate std;
// mod service {
//     use shaku::{module, Component, HasComponent, Interface};
//     use std::sync::Arc;
//     pub trait Logger: Interface {
//         fn log(&self, content: &str);
//     }
//     pub trait DateLogger: Interface {
//         fn log_date(&self);
//     }
//     # [ shaku ( interface = Logger ) ]
//     pub struct LoggerImpl;
//     impl<M: ::shaku::Module> ::shaku::Component<M> for LoggerImpl {
//         type Interface = dyn Logger;
//         type Parameters = LoggerImplParameters;
//         fn build(
//             context: &mut ::shaku::ModuleBuildContext<M>,
//             params: Self::Parameters,
//         ) -> Box<Self::Interface> {
//             Box::new(Self {})
//         }
//     }
//     pub struct LoggerImplParameters {}
//     impl ::std::default::Default for LoggerImplParameters {
//         #[allow(unreachable_code)]
//         fn default() -> Self {
//             Self {}
//         }
//     }
//     impl Logger for LoggerImpl {
//         fn log(&self, content: &str) {
//             {
//                 ::std::io::_print(::core::fmt::Arguments::new_v1(
//                     &["", "\n"],
//                     &match (&content,) {
//                         (arg0,) => [::core::fmt::ArgumentV1::new(
//                             arg0,
//                             ::core::fmt::Display::fmt,
//                         )],
//                     },
//                 ));
//             };
//         }
//     }
//     # [ shaku ( interface = DateLogger ) ]
//     pub struct DateLoggerImpl {
//         #[shaku(inject)]
//         logger: Arc<dyn Logger>,
//         today: String,
//         year: usize,
//     }
//     impl<M: ::shaku::Module + ::shaku::HasComponent<dyn Logger>> ::shaku::Component<M>
//         for DateLoggerImpl
//     {
//         type Interface = dyn DateLogger;
//         type Parameters = DateLoggerImplParameters;
//         fn build(
//             context: &mut ::shaku::ModuleBuildContext<M>,
//             params: Self::Parameters,
//         ) -> Box<Self::Interface> {
//             Box::new(Self {
//                 logger: M::build_component(context),
//                 today: params.today,
//                 year: params.year,
//             })
//         }
//     }
//     pub struct DateLoggerImplParameters {
//         pub today: String,
//         pub year: usize,
//     }
//     impl ::std::default::Default for DateLoggerImplParameters {
//         #[allow(unreachable_code)]
//         fn default() -> Self {
//             Self {
//                 today: Default::default(),
//                 year: Default::default(),
//             }
//         }
//     }
//     impl DateLogger for DateLoggerImpl {
//         fn log_date(&self) {
//             self.logger.log(&{
//                 let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
//                     &["Today is ", ", "],
//                     &match (&self.today, &self.year) {
//                         (arg0, arg1) => [
//                             ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
//                             ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
//                         ],
//                     },
//                 ));
//                 res
//             });
//         }
//     }
//     pub struct MyModule {
//         __di_component_0: ::std::sync::Arc<<LoggerImpl as ::shaku::Component<Self>>::Interface>,
//         __di_component_1: ::std::sync::Arc<<DateLoggerImpl as ::shaku::Component<Self>>::Interface>,
//     }
//     impl MyModule {
//         #[allow(bare_trait_objects)]
//         pub fn builder() -> ::shaku::ModuleBuilder<Self> {
//             ::shaku::ModuleBuilder::with_submodules(())
//         }
//     }
//     impl ::shaku::Module for MyModule {
//         #[allow(bare_trait_objects)]
//         type Submodules = ();
//         fn build(context: &mut ::shaku::ModuleBuildContext<Self>) -> Self {
//             Self {
//                 __di_component_0: <Self as ::shaku::HasComponent<
//                     <LoggerImpl as ::shaku::Component<Self>>::Interface,
//                 >>::build_component(context),
//                 __di_component_1: <Self as ::shaku::HasComponent<
//                     <DateLoggerImpl as ::shaku::Component<Self>>::Interface,
//                 >>::build_component(context),
//             }
//         }
//     }
//     impl ::shaku::HasComponent<<LoggerImpl as ::shaku::Component<Self>>::Interface> for MyModule {
//         fn build_component(
//             context: &mut ::shaku::ModuleBuildContext<Self>,
//         ) -> ::std::sync::Arc<<LoggerImpl as ::shaku::Component<Self>>::Interface> {
//             context.build_component::<LoggerImpl>()
//         }
//         fn resolve(&self) -> ::std::sync::Arc<<LoggerImpl as ::shaku::Component<Self>>::Interface> {
//             ::std::sync::Arc::clone(&self.__di_component_0)
//         }
//         fn resolve_ref(&self) -> &<LoggerImpl as ::shaku::Component<Self>>::Interface {
//             ::std::sync::Arc::as_ref(&self.__di_component_0)
//         }
//         fn resolve_mut(
//             &mut self,
//         ) -> ::std::option::Option<&mut <LoggerImpl as ::shaku::Component<Self>>::Interface>
//         {
//             ::std::sync::Arc::get_mut(&mut self.__di_component_0)
//         }
//     }
//     impl ::shaku::HasComponent<<DateLoggerImpl as ::shaku::Component<Self>>::Interface> for MyModule {
//         fn build_component(
//             context: &mut ::shaku::ModuleBuildContext<Self>,
//         ) -> ::std::sync::Arc<<DateLoggerImpl as ::shaku::Component<Self>>::Interface> {
//             context.build_component::<DateLoggerImpl>()
//         }
//         fn resolve(
//             &self,
//         ) -> ::std::sync::Arc<<DateLoggerImpl as ::shaku::Component<Self>>::Interface> {
//             ::std::sync::Arc::clone(&self.__di_component_1)
//         }
//         fn resolve_ref(&self) -> &<DateLoggerImpl as ::shaku::Component<Self>>::Interface {
//             ::std::sync::Arc::as_ref(&self.__di_component_1)
//         }
//         fn resolve_mut(
//             &mut self,
//         ) -> ::std::option::Option<&mut <DateLoggerImpl as ::shaku::Component<Self>>::Interface>
//         {
//             ::std::sync::Arc::get_mut(&mut self.__di_component_1)
//         }
//     }
// }
// mod controller {
//     use super::service::{DateLogger, MyModule};
//     use http::Method;
//     use shaku::HasComponent;
//
//     pub struct Handler;
//     impl Handler {
//         pub fn call(module: MyModule) {
//             let date_logger: &dyn DateLogger = module.resolve_ref();
//             Self::handle(date_logger)
//         }
//         pub fn method() -> Method {
//             Method::GET
//         }
//         fn handle(logger: &dyn DateLogger) {
//             logger.log_date()
//         }
//     }
// }
// extern crate test;
// #[cfg(test)]
// #[rustc_test_marker]
// pub const main: test::TestDescAndFn = test::TestDescAndFn {
//     desc: test::TestDesc {
//         name: test::StaticTestName("main"),
//         ignore: false,
//         allow_fail: false,
//         should_panic: test::ShouldPanic::No,
//         test_type: test::TestType::IntegrationTest,
//     },
//     testfn: test::StaticTestFn(|| test::assert_test_result(main())),
// };
// #[allow(dead_code)]
// fn main() {
//     let module = service::MyModule::builder()
//         .with_component_parameters::<service::DateLoggerImpl>(service::DateLoggerImplParameters {
//             today: "Jan 26".to_string(),
//             year: 2020,
//         })
//         .build();
//     let handler = controller::Handler::call(module);
// }
// #[main]
// pub fn main() -> () {
//     extern crate test;
//     test::test_main_static(&[&main])
// }
