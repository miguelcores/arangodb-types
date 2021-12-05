use serde::Deserialize;
use serde::Serialize;

use crate::aql::{AqlBuilder, AqlLimit, AqlReturn, AqlSort, AQL_DOCUMENT_ID};
use crate::traits::{PaginatedDocument, PaginatedDocumentField};
use crate::types::filters::{APIFilter, APIFilteringStatsConfig};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound = "F: PaginatedDocumentField")]
pub struct PaginatedRequest<F: PaginatedDocumentField> {
    #[serde(default)]
    pub sort_by: Vec<PaginatedSortByRequest<F>>,
    pub page: u64,
    pub rows_per_page: u64,
    #[serde(default)]
    pub filter_by: Option<APIFilter<F>>,
    #[serde(default)]
    pub fields_filter: Option<F::Document>,
    #[serde(default)]
    pub count_pages: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound = "F: PaginatedDocumentField")]
pub struct PaginatedSortByRequest<F: PaginatedDocumentField> {
    pub field: F,
    #[serde(default)]
    pub descending: bool,
}

impl<F: PaginatedDocumentField> PaginatedRequest<F> {
    // METHODS ----------------------------------------------------------------

    pub fn validate(&self, context: &F::Context) -> Result<(), anyhow::Error> {
        // Validate the sort_by.
        if self.sort_by.len() > 3 {
            return Err(anyhow::anyhow!(
                "The sortBy field cannot be longer than 3 fields"
            ));
        }

        for sort_by in &self.sort_by {
            if !sort_by.field.is_valid_for_sorting(context) {
                return Err(anyhow::anyhow!(
                    "The field '{}' is not valid for filtering",
                    serde_json::to_string(&sort_by.field).unwrap()
                ));
            }
        }

        // Validate the filter_by.
        if let Some(filter_by) = &self.filter_by {
            if filter_by.validate(context).is_err() {
                return Err(anyhow::anyhow!("Incorrect filterBy field"));
            }
        }

        // Validate rows.
        let minimum_rows = F::min_rows_per_page();
        if self.rows_per_page < minimum_rows {
            return Err(anyhow::anyhow!(
                "The minimum rows per page is {}. Current: {}",
                minimum_rows,
                self.rows_per_page
            ));
        }

        let maximum_rows = F::max_rows_per_page();
        if self.rows_per_page > maximum_rows {
            return Err(anyhow::anyhow!(
                "The maximum rows per page is {}. Current: {}",
                maximum_rows,
                self.rows_per_page
            ));
        }

        Ok(())
    }

    pub fn normalize(
        &mut self,
        filter_stats: &APIFilteringStatsConfig,
    ) -> Result<(), anyhow::Error> {
        if let Some(fields_filter) = &mut self.fields_filter {
            fields_filter.map_values_to_null();
        }

        if let Some(filter_by) = self.filter_by.take() {
            let filter_by = filter_by.normalize();
            let mut stats = APIFilteringStatsConfig::default();

            filter_by.calculate_stats(&mut stats);

            if stats.field_count > filter_stats.field_count
                || stats.const_count > filter_stats.const_count
                || stats.expression_count > filter_stats.expression_count
                || stats.function_count > filter_stats.function_count
            {
                return Err(anyhow::anyhow!(
                    "The filterBy property contains too many elements to be executed"
                ));
            }

            self.filter_by = Some(filter_by.normalize());
        }

        Ok(())
    }

    pub fn build_aql(self, collection: &str) -> Result<AqlBuilder, anyhow::Error> {
        // FOR i IN <collection>
        //      FILTER ..
        //      SORT ..
        //      LIMIT ..
        //      RETURN i
        let aql = AqlBuilder::new_for_in_collection(AQL_DOCUMENT_ID, collection);
        self.build_aql_using(aql)
    }

    pub fn build_aql_using(self, mut aql: AqlBuilder) -> Result<AqlBuilder, anyhow::Error> {
        // FOR i IN <collection>
        //      FILTER ..
        //      SORT ..
        //      LIMIT ..
        //      RETURN i

        // Filter part
        if let Some(filter_by) = &self.filter_by {
            let mut query = String::new();
            filter_by.build_aql(&mut query, &mut aql)?;

            aql.filter_step(query.into());
        }

        // Sort part
        if !self.sort_by.is_empty() {
            aql.sort_step(
                self.sort_by
                    .iter()
                    .map(|sorting| AqlSort {
                        expression: format!(
                            "{}.{}",
                            AQL_DOCUMENT_ID,
                            sorting.field.path_to_value()
                        )
                        .into(),
                        is_descending: sorting.descending,
                    })
                    .collect(),
            );
        }

        // Pagination
        aql.limit_step(AqlLimit {
            offset: Some(self.rows_per_page * self.page),
            count: self.rows_per_page,
        });
        aql.set_batch_size(Some(self.rows_per_page.min(100) as u32));
        aql.set_full_count(self.count_pages);
        aql.set_global_limit(self.rows_per_page);

        if let Some(fields) = self.fields_filter {
            let fields = fields.into_db_document();
            aql.return_step_with_fields(AQL_DOCUMENT_ID, &fields);
        } else {
            aql.return_step(AqlReturn::new_document());
        }

        Ok(aql)
    }
}
