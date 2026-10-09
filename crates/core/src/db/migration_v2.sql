-- v2：供应商模型列表（JSON 字符串数组；用于详情页模型管理与默认模型选择）
ALTER TABLE ai_providers ADD COLUMN models TEXT NOT NULL DEFAULT '[]';
