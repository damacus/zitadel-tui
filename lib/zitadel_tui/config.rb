# frozen_string_literal: true

require 'tty-config'
require 'yaml'

module ZitadelTui
  class Config
    SA_KEY_FILE = '/tmp/zitadel-sa.json'
    SA_SECRET_NAME = 'zitadel-admin-sa'
    SA_SECRET_NAMESPACE = 'authentication'
    SA_SECRET_KEY = 'zitadel-admin-sa.json'
    PAT_SECRET_NAME = 'zitadel-admin-sa-pat'
    PAT_SECRET_KEY = 'pat'
    GOOGLE_IDP_SECRET = 'zitadel-google-idp'

    attr_reader :config

    def initialize
      @config = TTY::Config.new
      @config.filename = 'zitadel-tui'
      @config.extname = '.yml'
      @config.append_path(Dir.home)
      @config.append_path(Dir.pwd)

      set_defaults
      load_config
    end

    def zitadel_url
      @config.fetch(:zitadel_url)
    end

    def zitadel_url=(value)
      @config.set(:zitadel_url, value: value)
    end

    def project_id
      @config.fetch(:project_id)
    end

    def project_id=(value)
      @config.set(:project_id, value: value)
    end

    def sa_key_file
      @config.fetch(:sa_key_file, default: SA_KEY_FILE)
    end

    def apps_config_file
      @config.fetch(:apps_config_file)
    end

    def apps_config_file=(value)
      @config.set(:apps_config_file, value: value)
    end

    def configured?
      !zitadel_url.nil? && !zitadel_url.empty?
    end

    def predefined_apps
      return {} unless apps_config_file && File.exist?(apps_config_file)

      yaml = YAML.safe_load_file(apps_config_file, symbolize_names: true)
      return {} unless yaml && yaml[:apps]

      yaml[:apps].transform_values do |app|
        {
          redirect_uris: app[:redirect_uris] || [],
          public: app[:public] || false
        }
      end
    end

    def predefined_users
      return [] unless apps_config_file && File.exist?(apps_config_file)

      yaml = YAML.safe_load_file(apps_config_file, symbolize_names: true)
      return [] unless yaml && yaml[:users]

      yaml[:users].map do |user|
        {
          email: user[:email],
          first_name: user[:first_name],
          last_name: user[:last_name],
          admin: user[:admin] || false
        }
      end
    end

    def save
      @config.write(force: true)
    end

    private

    def set_defaults
      @config.set(:sa_key_file, value: SA_KEY_FILE)
    end

    def load_config
      @config.read if @config.exist?
    rescue TTY::Config::ReadError
      # Config file doesn't exist yet, use defaults
    end
  end
end
