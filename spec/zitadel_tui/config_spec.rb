# frozen_string_literal: true

require 'spec_helper'
require 'tempfile'

RSpec.describe ZitadelTui::Config do
  subject(:config) { described_class.new }

  describe '#zitadel_url' do
    it 'returns nil when not configured' do
      expect(config.zitadel_url).to be_nil
    end
  end

  describe '#configured?' do
    it 'returns false when zitadel_url is not set' do
      expect(config.configured?).to be false
    end

    it 'returns true when zitadel_url is set' do
      config.zitadel_url = 'https://zitadel.example.com'
      expect(config.configured?).to be true
    end
  end

  describe '#sa_key_file' do
    it 'returns the default key file path' do
      expect(config.sa_key_file).to eq('/tmp/zitadel-sa.json')
    end
  end

  describe '#predefined_apps' do
    it 'returns empty hash when no apps config file is set' do
      expect(config.predefined_apps).to eq({})
    end

    it 'returns empty hash when apps config file does not exist' do
      config.apps_config_file = '/nonexistent/apps.yml'
      expect(config.predefined_apps).to eq({})
    end

    it 'parses apps from YAML file' do
      Tempfile.create(['apps', '.yml']) do |f|
        f.write(<<~YAML)
          apps:
            grafana:
              redirect_uris:
                - https://grafana.example.com/callback
              public: false
            mealie:
              redirect_uris:
                - https://mealie.example.com/login
              public: true
        YAML
        f.flush

        config.apps_config_file = f.path
        apps = config.predefined_apps

        expect(apps[:grafana][:redirect_uris]).to eq(['https://grafana.example.com/callback'])
        expect(apps[:grafana][:public]).to be false
        expect(apps[:mealie][:public]).to be true
      end
    end
  end
end
