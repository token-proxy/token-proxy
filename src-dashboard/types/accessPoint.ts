export interface ProviderOption {
  id: string;
  name: string;
}

export interface AccountOption {
  id: string;
  provider_id: string;
  name: string;
  api_key_suffix: string;
  status: string;
}

export interface ModelMapping {
  source_model: string;
  target_model: string;
}

export interface AccessPoint {
  id: string;
  name: string;
  short_code: string;
  provider_id: string;
  account_id: string;
  api_type: string;
  model_mappings: ModelMapping[];
  access_url: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface AccessPointFormData {
  name: string;
  short_code: string;
  provider_id: string | undefined;
  account_id: string | undefined;
  api_type: string;
}
