use crate::{chemistry::Rule, parser::parse_rules_file};
use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    reflect::TypeUuid,
};
use simple_error::SimpleError;
use std::str;

#[derive(TypeUuid)]
#[uuid = "e8289333-2ced-4559-8ab3-d24b65f8cad4"]
pub(crate) struct RulesAsset(pub(crate) Vec<Rule>);

#[derive(Default)]
pub(crate) struct RulesAssetLoader;

impl AssetLoader for RulesAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let rules_file = str::from_utf8(bytes)?;
            let rules = parse_rules_file(rules_file).map_err(SimpleError::new)?;
            load_context.set_default_asset(LoadedAsset::new(RulesAsset(rules)));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["rules"]
    }
}
