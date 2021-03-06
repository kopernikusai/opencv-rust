use std::{
	borrow::Cow,
	fmt,
};

use clang::{
	Entity,
	Type,
};
use maplit::hashmap;
use once_cell::sync::Lazy;

use crate::{
	CompiledInterpolation,
	DefaultElement,
	DefinitionLocation,
	DependentTypeMode,
	Element,
	EntityElement,
	GeneratedElement,
	GeneratorEnv,
	ReturnTypeWrapper,
	StrExt,
	type_ref::TemplateArg,
	TypeRef,
};

#[derive(Clone)]
pub struct Vector<'tu, 'g> {
	type_ref: Type<'tu>,
	gen_env: &'g GeneratorEnv<'tu>,
}

impl<'tu, 'g> Vector<'tu, 'g> {
	pub fn new(type_ref: Type<'tu>, gen_env: &'g GeneratorEnv<'tu>) -> Self {
		Self { type_ref, gen_env }
	}

	fn is_data_type(&self, type_ref: &TypeRef) -> bool {
		type_ref.is_data_type() || type_ref.as_vector().map_or(false, |v| v.element_type().is_data_type())
	}

	pub fn type_ref(&self) -> TypeRef<'tu, 'g> {
		TypeRef::new(self.type_ref, self.gen_env)
	}

	pub fn element_type(&self) -> TypeRef<'tu, 'g> {
		self.type_ref().template_args().into_iter()
			.find_map(|a| if let TemplateArg::Typename(type_ref) = a {
				Some(type_ref)
			} else {
				None
			}).expect("vector template argument list is empty")
	}

	pub fn dependent_types(&self) -> Vec<Box<dyn GeneratedElement + 'g>> {
		let element_type = self.element_type();
		let needs_ioa = self.is_data_type(&element_type);
		let needs_char_ptr = element_type.is_string();
		let mut out = element_type.dependent_types_with_mode(DependentTypeMode::ForReturn(DefinitionLocation::Type));
		out.reserve(2 + if needs_ioa { 3 } else { 0 } + if needs_char_ptr { 1 } else { 0 });
		out.push(Box::new(ReturnTypeWrapper::new(element_type.canonical_clang(), self.gen_env, DefinitionLocation::Type)));
		out.push(Box::new(ReturnTypeWrapper::new(self.type_ref().canonical_clang(), self.gen_env, DefinitionLocation::Module)));
		if needs_ioa {
			out.push(Box::new(ReturnTypeWrapper::new(
				TypeRef::new(self.gen_env.resolve_type("cv::_InputArray").expect("Can't resolve _InputArray"), self.gen_env),
				self.gen_env,
				DefinitionLocation::Custom(element_type.rust_module().into_owned()),
			)));
			out.push(Box::new(ReturnTypeWrapper::new(
				TypeRef::new(self.gen_env.resolve_type("cv::_OutputArray").expect("Can't resolve _OutputArray"), self.gen_env),
				self.gen_env,
				DefinitionLocation::Custom(element_type.rust_module().into_owned()),
			)));
			out.push(Box::new(ReturnTypeWrapper::new(
				TypeRef::new(self.gen_env.resolve_type("cv::_InputOutputArray").expect("Can't resolve _InputOutputArray"), self.gen_env),
				self.gen_env,
				DefinitionLocation::Custom(element_type.rust_module().into_owned()),
			)));
		}
		if needs_char_ptr {
			out.push(Box::new(ReturnTypeWrapper::new(
				TypeRef::new(self.gen_env.resolve_type("const char*").expect("Can't resolve _InputOutputArray"), self.gen_env),
				self.gen_env,
				DefinitionLocation::Custom(element_type.rust_module().into_owned()),
			)));
		}
		out
	}
}

impl<'tu> EntityElement<'tu> for Vector<'tu, '_> {
	fn entity(&self) -> Entity<'tu> {
		self.type_ref.get_declaration().expect("Can't get declaration")
	}
}

impl Element for Vector<'_, '_> {
	fn is_ignored(&self) -> bool {
		// workaround for race when type with std::string gets generated first
		// we only want vector<cv::String> because it's more compatible across OpenCV versions
		DefaultElement::is_ignored(self) || self.element_type().is_ignored()
	}

	fn is_system(&self) -> bool {
		DefaultElement::is_system(self)
	}

	fn is_public(&self) -> bool {
		DefaultElement::is_public(self)
	}

	fn usr(&self) -> Cow<str> {
		DefaultElement::usr(self)
	}

	fn rendered_doc_comment_with_prefix(&self, prefix: &str, opencv_version: &str) -> String {
		DefaultElement::rendered_doc_comment_with_prefix(self, prefix, opencv_version)
	}

	fn cpp_namespace(&self) -> Cow<str> {
		DefaultElement::cpp_namespace(self)
	}

	fn cpp_localname(&self) -> Cow<str> {
		DefaultElement::cpp_localname(self)
	}

	fn rust_module(&self) -> Cow<str> {
		self.element_type().rust_module().into_owned().into()
	}

	fn rust_leafname(&self) -> Cow<str> {
		format!("VectorOf{typ}", typ = self.element_type().rust_safe_id()).into()
	}

	fn rust_localname(&self) -> Cow<str> {
		DefaultElement::rust_localname(self)
	}

	fn rust_fullname(&self) -> Cow<str> {
		format!("types::{}", self.rust_localname()).into()
	}
}

impl GeneratedElement for Vector<'_, '_> {
	fn element_safe_id(&self) -> String {
		format!("{}-{}", self.rust_module(), self.rust_localname())
	}

	fn gen_rust(&self, _opencv_version: &str) -> String {
		static COMMON_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_common.tpl.rs").compile_interpolation()
		);

		static METHODS_BOXED_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_methods_boxed.tpl.rs").compile_interpolation()
		);

		static METHODS_STRING_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_methods_string.tpl.rs").compile_interpolation()
		);

		static METHODS_NON_BOXED_COMMON_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_methods_non_boxed_common.tpl.rs").compile_interpolation()
		);

		static METHODS_SIMPLE_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_methods_simple.tpl.rs").compile_interpolation()
		);

		static METHODS_NON_BOXED_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_methods_non_boxed.tpl.rs").compile_interpolation()
		);

		static METHODS_COPY_NON_BOOL_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_methods_copy_non_bool.tpl.rs").compile_interpolation()
		);

		static INHERENT_COPY_NON_BOOL_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_inherent_copy_non_bool.tpl.rs").compile_interpolation()
		);

		static INPUT_OUTPUT_ARRAY_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/rust_input_output_array.tpl.rs").compile_interpolation()
		);

		let vec_type = self.type_ref();
		let element_type = self.element_type();
		let mut inter_vars = hashmap! {
			"rust_local" => vec_type.rust_local(),
			"rust_extern" => vec_type.rust_extern(),
			"cpp_full" => vec_type.cpp_full(),
			"cpp_extern" => vec_type.cpp_extern(),
			"inner_rust_extern" => element_type.rust_extern(),
			"inner_rust_local" => element_type.rust_local(),
			"inner_canonical_rust_local" => element_type.canonical().rust_local().into_owned().into(),
			"inner_rust_full" => element_type.rust_full(),
			"rust_return_wrapper_type" => element_type.rust_extern_return_wrapper_full(),
		};
		let mut vector_methods = String::new();
		let mut inherent_methods = String::new();
		let mut impls = String::new();
		if element_type.is_by_ptr() {
			vector_methods += &METHODS_BOXED_TPL.interpolate(&inter_vars);
		} else if element_type.is_string() {
			vector_methods += &METHODS_STRING_TPL.interpolate(&inter_vars);
		} else {
			vector_methods += &METHODS_NON_BOXED_COMMON_TPL.interpolate(&inter_vars);
			if element_type.as_simple_class().is_some() {
				vector_methods += &METHODS_SIMPLE_TPL.interpolate(&inter_vars);
			} else {
				vector_methods += &METHODS_NON_BOXED_TPL.interpolate(&inter_vars);
			}
			if element_type.is_copy() && !element_type.is_bool() {
				vector_methods += &METHODS_COPY_NON_BOOL_TPL.interpolate(&inter_vars);
				inherent_methods += &INHERENT_COPY_NON_BOOL_TPL.interpolate(&inter_vars);
			}
		}
		if self.is_data_type(&element_type) {
			impls += &INPUT_OUTPUT_ARRAY_TPL.interpolate(&inter_vars);
		}

		inter_vars.insert("vector_methods", vector_methods.into());
		inter_vars.insert("inherent_methods", inherent_methods.into());
		inter_vars.insert("impls", impls.into());
		COMMON_TPL.interpolate(&inter_vars)
	}

	fn gen_cpp(&self) -> String {
		static COMMON_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/cpp_common.tpl.cpp").compile_interpolation()
		);

		static METHODS_BOXED_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/cpp_methods_boxed.tpl.cpp").compile_interpolation()
		);

		static METHODS_STRING_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/cpp_methods_string.tpl.cpp").compile_interpolation()
		);

		static METHODS_NON_BOXED_COMMON_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/cpp_methods_non_boxed_common.tpl.cpp").compile_interpolation()
		);

		static METHODS_SIMPLE_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/cpp_methods_simple.tpl.cpp").compile_interpolation()
		);

		static METHODS_NON_BOXED_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/cpp_methods_non_boxed.tpl.cpp").compile_interpolation()
		);

		static METHODS_COPY_NON_BOOL_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/cpp_methods_copy_non_bool.tpl.cpp").compile_interpolation()
		);

		static INPUT_OUTPUT_ARRAY_TPL: Lazy<CompiledInterpolation> = Lazy::new(
			|| include_str!("../tpl/vector/cpp_input_output_aray.tpl.cpp").compile_interpolation()
		);

		let vec_type = self.type_ref();
		let element_type = self.element_type();
		let mut inter_vars = hashmap! {
			"rust_local" => vec_type.rust_local(),
			"cpp_full" => vec_type.cpp_full(),
			"cpp_extern" => vec_type.cpp_extern(),
			"cpp_extern_return" => vec_type.cpp_extern_return(),
			"inner_cpp_full" => element_type.cpp_full(),
			"inner_cpp_extern" => element_type.cpp_extern(),
			"inner_cpp_extern_return" => element_type.cpp_extern_return(),
			"cpp_return_wrapper_type" => element_type.cpp_extern_return_wrapper_full(),
			"call_arg" => element_type.cpp_arg_func_call("val").into(),
		};

		let mut exports = String::new();
		if element_type.is_by_ptr() {
			exports += &METHODS_BOXED_TPL.interpolate(&inter_vars);
		} else if element_type.is_string() {
			exports += &METHODS_STRING_TPL.interpolate(&inter_vars);
		} else {
			exports += &METHODS_NON_BOXED_COMMON_TPL.interpolate(&inter_vars);
			if element_type.as_simple_class().is_some() {
				exports += &METHODS_SIMPLE_TPL.interpolate(&inter_vars);
			} else {
				exports += &METHODS_NON_BOXED_TPL.interpolate(&inter_vars);
			}
			if element_type.is_copy() && !element_type.is_bool() {
				exports += &METHODS_COPY_NON_BOOL_TPL.interpolate(&inter_vars);
			}
		}
		if self.is_data_type(&element_type) {
			exports += &INPUT_OUTPUT_ARRAY_TPL.interpolate(&inter_vars);
		}
		inter_vars.insert("exports", exports.into());

		COMMON_TPL.interpolate(&inter_vars)
	}
}

impl fmt::Display for Vector<'_, '_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.entity().get_display_name().expect("Can't get display name"))
	}
}

impl fmt::Debug for Vector<'_, '_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut debug_struct = f.debug_struct("Vector");
		self.update_debug_struct(&mut debug_struct)
			.field("export_config", &self.gen_env.get_export_config(self.entity()))
			.field("element_type", &self.element_type())
			.finish()
	}
}
