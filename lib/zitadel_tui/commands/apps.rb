# frozen_string_literal: true

module ZitadelTui
  module Commands
    class Apps
      def initialize(client:, ui:)
        @client = client
        @ui = ui
      end

      def predefined_apps
        @client.config.predefined_apps
      end

      def menu
        loop do
          @ui.clear
          @ui.header('OIDC Application Management')

          choice = @ui.select_menu('What would you like to do?', [
                                     { name: '📋 List all applications', value: :list },
                                     { name: '➕ Create new application', value: :create },
                                     { name: '🔄 Regenerate client secret', value: :regenerate },
                                     { name: '🗑️  Delete application', value: :delete },
                                     { name: '🚀 Quick setup (predefined apps)', value: :quick_setup },
                                     { name: '← Back to main menu', value: :back }
                                   ])

          case choice
          when :list then list_apps
          when :create then create_app
          when :regenerate then regenerate_secret
          when :delete then delete_app
          when :quick_setup then quick_setup
          when :back then break
          end

          @ui.press_any_key unless choice == :back
        end
      end

      private

      def ensure_project
        @project ||= @ui.spinner('Fetching project...') { @client.get_default_project }
        @project_id = @project['id']
      end

      def list_apps
        ensure_project
        @ui.subheader('OIDC Applications')

        apps = @ui.spinner('Fetching applications...') { @client.list_apps(@project_id) }

        if apps.empty?
          @ui.warning('No applications found')
          return
        end

        rows = apps.map do |app|
          oidc = app['oidcConfig']
          [
            app['name'],
            app['id'],
            oidc&.dig('clientId') || 'N/A',
            if oidc
              oidc['authMethodType'] == 'OIDC_AUTH_METHOD_TYPE_NONE' ? 'Public' : 'Confidential'
            else
              'N/A'
            end,
            app['state']
          ]
        end

        @ui.table(%w[Name AppID ClientID Type State], rows)
      end

      def create_app
        ensure_project
        @ui.subheader('Create OIDC Application')

        use_predefined = @ui.yes?('Use a predefined application template?')

        if use_predefined
          create_from_template
        else
          create_custom_app
        end
      end

      def create_from_template
        apps = predefined_apps
        if apps.empty?
          @ui.warning('No predefined applications configured. Add apps to your apps.yml file.')
          return
        end

        available = apps.keys.map(&:to_s) - existing_app_names
        if available.empty?
          @ui.warning('All predefined applications already exist')
          return
        end

        app_name = @ui.select_menu('Select application template:', available)
        template = apps[app_name.to_sym]

        @ui.info("Creating #{app_name}...")
        @ui.info("Redirect URIs: #{template[:redirect_uris].join(', ')}")
        @ui.info("Type: #{template[:public] ? 'Public' : 'Confidential'}")

        return unless @ui.yes?('Proceed with creation?')

        result = @ui.spinner("Creating #{app_name}...") do
          @client.create_oidc_app(@project_id, app_name, template[:redirect_uris], public: template[:public])
        end

        display_credentials(app_name, result, template[:public])
      end

      def create_custom_app
        data = @ui.collect do
          key(:name).ask('Application name:', required: true)
          key(:redirect_uris).ask('Redirect URIs (comma-separated):', required: true) do |q|
            q.convert ->(input) { input.split(',').map(&:strip) }
          end
          key(:public).yes?('Is this a public client (no secret)?')
        end

        result = @ui.spinner("Creating #{data[:name]}...") do
          @client.create_oidc_app(@project_id, data[:name], data[:redirect_uris], public: data[:public])
        end

        display_credentials(data[:name], result, data[:public])
      end

      def regenerate_secret
        ensure_project
        @ui.subheader('Regenerate Client Secret')

        apps = @ui.spinner('Fetching applications...') { @client.list_apps(@project_id) }
        confidential_apps = apps.reject do |app|
          app.dig('oidcConfig', 'authMethodType') == 'OIDC_AUTH_METHOD_TYPE_NONE'
        end

        if confidential_apps.empty?
          @ui.warning('No confidential applications found')
          return
        end

        choices = confidential_apps.map do |app|
          { name: "#{app['name']} (#{app.dig('oidcConfig', 'clientId')})", value: app }
        end

        selected = @ui.select_menu('Select application:', choices)

        @ui.warning("This will invalidate the current secret for #{selected['name']}")
        return unless @ui.yes?('Are you sure you want to regenerate the secret?')

        result = @ui.spinner('Regenerating secret...') do
          @client.regenerate_secret(@project_id, selected['id'])
        end

        @ui.success('Secret regenerated successfully!')
        @ui.credentials_box("#{selected['name']} - New Credentials", {
                              'Client ID' => selected.dig('oidcConfig', 'clientId'),
                              'Client Secret' => result['clientSecret']
                            })
      end

      def delete_app
        ensure_project
        @ui.subheader('Delete Application')

        apps = @ui.spinner('Fetching applications...') { @client.list_apps(@project_id) }

        if apps.empty?
          @ui.warning('No applications found')
          return
        end

        choices = apps.map do |app|
          { name: "#{app['name']} (#{app['id']})", value: app }
        end

        selected = @ui.select_menu('Select application to delete:', choices)

        @ui.error("WARNING: This will permanently delete #{selected['name']}")
        return unless @ui.yes?('Are you absolutely sure?', default: false)

        @ui.spinner("Deleting #{selected['name']}...") do
          @client.delete_app(@project_id, selected['id'])
        end

        @ui.success("Application #{selected['name']} deleted successfully")
      end

      def quick_setup
        ensure_project
        @ui.subheader('Quick Setup - Predefined Applications')

        apps = predefined_apps
        if apps.empty?
          @ui.warning('No predefined applications configured.')
          @ui.info('Create an apps.yml file with your application definitions.')
          @ui.info('See README.md for configuration format.')
          return
        end

        existing = existing_app_names
        available = apps.keys.map(&:to_s) - existing

        if available.empty?
          @ui.success('All predefined applications already exist!')
          return
        end

        @ui.info("Existing apps: #{existing.join(', ')}") unless existing.empty?
        @ui.info("Available to create: #{available.join(', ')}")

        selected = @ui.multi_select_menu('Select applications to create:', available)

        selected.each do |app_name|
          template = apps[app_name.to_sym]
          result = @ui.spinner("Creating #{app_name}...") do
            @client.create_oidc_app(@project_id, app_name, template[:redirect_uris], public: template[:public])
          end

          display_credentials(app_name, result, template[:public])
          @ui.newline
        end

        @ui.success('Quick setup complete!')
      end

      def existing_app_names
        apps = @client.list_apps(@project_id)
        apps.map { |a| a['name'] }
      end

      def display_credentials(name, result, is_public)
        @ui.success("Application #{name} created successfully!")

        credentials = { 'Client ID' => result['clientId'] }
        credentials['Client Secret'] = result['clientSecret'] unless is_public

        @ui.credentials_box("#{name} - Credentials", credentials)
      end
    end
  end
end
