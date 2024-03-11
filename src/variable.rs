// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_ast::swc::ast::Pat;
use deno_graph::symbols::EsModuleInfo;
use deno_graph::symbols::SymbolNodeRef;
use serde::Deserialize;
use serde::Serialize;

use crate::ts_type::infer_simple_ts_type_from_var_decl;
use crate::ts_type::TsTypeDef;
use crate::ts_type::TsTypeDefKind;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VariableDef {
  pub ts_type: Option<TsTypeDef>,
  pub kind: deno_ast::swc::ast::VarDeclKind,
}

pub fn get_docs_for_var_declarator(
  module_info: &EsModuleInfo,
  var_decl: &deno_ast::swc::ast::VarDecl,
  var_declarator: &deno_ast::swc::ast::VarDeclarator,
) -> Vec<(String, VariableDef)> {
  let mut items = Vec::<(String, VariableDef)>::new();
  let ref_name: Option<deno_ast::swc::ast::Id> =
    var_declarator.init.as_ref().and_then(|init| {
      if let deno_ast::swc::ast::Expr::Ident(ident) = &**init {
        Some(ident.to_id())
      } else {
        None
      }
    });

  let maybe_ts_type_ann = match &var_declarator.name {
    Pat::Ident(ident) => ident.type_ann.as_ref(),
    Pat::Object(pat) => pat.type_ann.as_ref(),
    Pat::Array(pat) => pat.type_ann.as_ref(),
    _ => None,
  };
  let maybe_ts_type = maybe_ts_type_ann
    .map(|def| TsTypeDef::new(module_info.source(), &def.type_ann))
    .or_else(|| {
      if let Some(ref_name) = ref_name {
        module_info.symbol_from_swc(&ref_name).and_then(|symbol| {
          // todo(dsherret): it would be better to go to the declaration
          // here, which is somewhat trivial with type tracing.
          for decl in symbol.decls() {
            if let Some(SymbolNodeRef::Var(_, var_declarator, _)) =
              decl.maybe_node()
            {
              if let Pat::Ident(ident) = &var_declarator.name {
                if let Some(type_ann) = &ident.type_ann {
                  return Some(TsTypeDef::new(
                    module_info.source(),
                    &type_ann.type_ann,
                  ));
                }
              }
            }
            let maybe_type_ann = infer_simple_ts_type_from_var_decl(
              module_info.source(),
              var_declarator,
              var_decl.kind == deno_ast::swc::ast::VarDeclKind::Const,
            );
            if let Some(type_ann) = maybe_type_ann {
              return Some(type_ann);
            }
          }
          None
        })
      } else {
        None
      }
    })
    .or_else(|| {
      infer_simple_ts_type_from_var_decl(
        module_info.source(),
        var_declarator,
        var_decl.kind == deno_ast::swc::ast::VarDeclKind::Const,
      )
    });

  match &var_declarator.name {
    Pat::Ident(ident) => {
      let var_name = ident.id.sym.to_string();
      let variable_def = VariableDef {
        ts_type: maybe_ts_type,
        kind: var_decl.kind,
      };
      items.push((var_name, variable_def));
    }
    Pat::Object(obj) => {
      let mut reached_rest = false;
      for prop in &obj.props {
        assert!(!reached_rest, "object rest is always last");
        let (name, reassign_name, rest_type_ann) = match prop {
          deno_ast::swc::ast::ObjectPatProp::KeyValue(kv) => (
            crate::params::prop_name_to_string(module_info.source(), &kv.key),
            match &*kv.value {
              Pat::Ident(ident) => Some(ident.sym.to_string()),
              _ => None, // TODO(@crowlKats): cover other cases?
            },
            None,
          ),
          deno_ast::swc::ast::ObjectPatProp::Assign(assign) => {
            (assign.key.sym.to_string(), None, None)
          }
          deno_ast::swc::ast::ObjectPatProp::Rest(rest) => {
            reached_rest = true;

            (
              match &*rest.arg {
                Pat::Ident(ident) => ident.sym.to_string(),
                _ => continue, // TODO(@crowlKats): cover other cases?
              },
              None,
              rest.type_ann.as_ref(),
            )
          }
        };

        let ts_type = if !reached_rest {
          maybe_ts_type.as_ref().and_then(|ts_type| {
            ts_type.type_literal.as_ref().and_then(|type_literal| {
              type_literal.properties.iter().find_map(|property| {
                if property.name == name {
                  property.ts_type.clone()
                } else {
                  None
                }
              })
            })
          })
        } else {
          rest_type_ann.map(|type_ann| {
            TsTypeDef::new(module_info.source(), &type_ann.type_ann)
          })
        };

        let variable_def = VariableDef {
          ts_type,
          kind: var_decl.kind,
        };
        items.push((reassign_name.unwrap_or(name), variable_def));
      }
    }
    Pat::Array(arr) => {
      let mut reached_rest = false;
      for (i, elem) in arr.elems.iter().enumerate() {
        assert!(!reached_rest, "object rest is always last");
        let Some(elem) = elem else {
          continue;
        };

        let (name, rest_type_ann) = match elem {
          Pat::Ident(ident) => (ident.sym.to_string(), None),
          Pat::Rest(rest) => {
            reached_rest = true;
            (
              match &*rest.arg {
                Pat::Ident(ident) => ident.sym.to_string(),
                _ => continue, // TODO(@crowlKats): cover other cases?
              },
              rest.type_ann.as_ref(),
            )
          }
          // TODO(@crowlKats): maybe handle assign pat?
          _ => continue,
        };

        let ts_type = if !reached_rest {
          maybe_ts_type.as_ref().and_then(|ts_type| {
            match ts_type.kind.as_ref()? {
              TsTypeDefKind::Array => Some(*ts_type.array.clone().unwrap()),
              TsTypeDefKind::Tuple => ts_type
                .tuple
                .as_ref()
                .unwrap()
                .get(i)
                .map(|def| def.clone()),
              _ => None,
            }
          })
        } else {
          rest_type_ann
            .map(|type_ann| {
              TsTypeDef::new(module_info.source(), &type_ann.type_ann)
            })
            .or_else(|| {
              maybe_ts_type.as_ref().and_then(|ts_type| {
                if ts_type.kind == Some(TsTypeDefKind::Array) {
                  Some(ts_type.clone())
                } else {
                  None
                }
              })
            })
        };

        let variable_def = VariableDef {
          ts_type,
          kind: var_decl.kind,
        };
        items.push((name, variable_def));
      }
    }
    _ => (),
  }
  items
}
