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

  describe '#predefined_users' do
    it 'returns empty array when no config file is set' do
      expect(config.predefined_users).to eq([])
    end

    it 'returns empty array when config file does not exist' do
      config.apps_config_file = '/nonexistent/apps.yml'
      expect(config.predefined_users).to eq([])
    end

    it 'parses users from YAML file' do
      Tempfile.create(['apps', '.yml']) do |f|
        f.write(<<~YAML)
          users:
            - email: admin@example.com
              first_name: Admin
              last_name: User
              admin: true
            - email: user@example.com
              first_name: Regular
              last_name: User
        YAML
        f.flush

        config.apps_config_file = f.path
        users = config.predefined_users

        expect(users.length).to eq(2)
        expect(users[0][:email]).to eq('admin@example.com')
        expect(users[0][:first_name]).to eq('Admin')
        expect(users[0][:admin]).to be true
        expect(users[1][:email]).to eq('user@example.com')
        expect(users[1][:admin]).to be false
      end
    end
  end
end
