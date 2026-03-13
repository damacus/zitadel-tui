# frozen_string_literal: true

require 'json'
require 'base64'

module ZitadelTui
  module Commands
    class Idp
      def initialize(client:, ui:)
        @client = client
        @ui = ui
      end

      def menu
        loop do
          @ui.clear
          @ui.header('Identity Provider Configuration')

          choice = @ui.select_menu('What would you like to do?', [
                                     { name: '📋 List configured IDPs', value: :list },
                                     { name: '🔗 Configure Google IDP', value: :configure_google },
                                     { name: '← Back to main menu', value: :back }
                                   ])

          case choice
          when :list then list_idps
          when :configure_google then configure_google
          when :back then break
          end

          @ui.press_any_key unless choice == :back
        end
      end

      private

      def list_idps
        @ui.subheader('Configured Identity Providers')

        idps = @ui.spinner('Fetching IDPs...') { @client.list_idps }

        if idps.empty?
          @ui.warning('No identity providers configured')
          return
        end

        rows = idps.map do |idp|
          [
            idp['name'],
            idp['id'],
            idp['type'] || 'Unknown',
            idp['state']
          ]
        end

        @ui.table(%w[Name ID Type State], rows)
      end

      def configure_google
        @ui.subheader('Configure Google Identity Provider')

        @ui.info('This will configure Google as an identity provider for Zitadel.')
        @ui.info('Users will be able to sign in with their Google accounts.')
        @ui.newline

        source = @ui.select_menu('Where should I get the Google OAuth credentials?', [
                                   { name: '🔐 From Kubernetes secret (zitadel-google-idp)', value: :kubernetes },
                                   { name: '✏️  Enter manually', value: :manual }
                                 ])

        credentials = case source
                      when :kubernetes then fetch_google_credentials_from_k8s
                      when :manual then get_manual_credentials
                      end

        return if credentials.nil?

        @ui.info("Client ID: #{credentials[:client_id]}")
        @ui.newline

        return unless @ui.yes?('Configure Google IDP with these credentials?')

        result = @ui.spinner('Configuring Google IDP...') do
          @client.add_google_idp(
            client_id: credentials[:client_id],
            client_secret: credentials[:client_secret]
          )
        end

        @ui.success('Google IDP configured successfully!')
        @ui.info("IDP ID: #{result['id']}")
        @ui.newline
        @ui.info('Users can now:')
        @ui.info('  • Register with username/password')
        @ui.info('  • Login with Google account')
        @ui.info('  • Link Google account to existing local account')
      rescue ZitadelTui::ApiError => e
        if e.message.include?('already exists')
          @ui.warning('Google IDP is already configured')
        else
          @ui.error("Failed to configure Google IDP: #{e.message}")
        end
      end

      def fetch_google_credentials_from_k8s
        @ui.spinner('Fetching credentials from Kubernetes...') do
          # Performance: Batch kubectl secret retrievals.
          # Spawning external processes (like kubectl) is a significant bottleneck.
          # By using `kubectl get secret ... -o json` once and parsing the output,
          # we avoid multiple process spawns and reduce network API overhead.
          cmd = TTY::Command.new(printer: :null)
          result = cmd.run('kubectl', 'get', 'secret', Config::GOOGLE_IDP_SECRET,
                           '-n', Config::SA_SECRET_NAMESPACE, '-o', 'json',
                           only_output_on_error: true).out.strip

          raise "Secret #{Config::GOOGLE_IDP_SECRET} not found" if result.empty?

          parsed_secret = JSON.parse(result)
          data = parsed_secret['data'] || {}

          client_id_encoded = data['client-id']
          client_secret_encoded = data['client-secret']

          raise 'Key client-id not found in secret' if client_id_encoded.nil? || client_id_encoded.empty?
          raise 'Key client-secret not found in secret' if client_secret_encoded.nil? || client_secret_encoded.empty?

          client_id = Base64.decode64(client_id_encoded)
          client_secret = Base64.decode64(client_secret_encoded)

          { client_id: client_id, client_secret: client_secret }
        end
      rescue StandardError => e
        @ui.error("Failed to fetch credentials: #{e.message}")
        @ui.info('You can enter credentials manually instead.')

        return nil unless @ui.yes?('Enter credentials manually?')

        get_manual_credentials
      end

      def get_manual_credentials
        @ui.collect do
          key(:client_id).ask('Google Client ID:', required: true)
          key(:client_secret).mask('Google Client Secret:', required: true)
        end
      end
    end
  end
end
